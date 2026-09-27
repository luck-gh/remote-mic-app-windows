//! Fixed SayAllInput package maintenance. No caller-supplied path or command.
use super::*;
use std::os::windows::{
    ffi::OsStrExt,
    fs::{MetadataExt, OpenOptionsExt},
    io::AsRawHandle,
};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{GetLastError, HANDLE, HWND};
use windows::Win32::Security::{Cryptography::Catalog::*, WinTrust::*};

include!("../../../drivers/SayAllInput/package_identity.rs");
const INF: &[u8] = include_bytes!("../../../drivers/SayAllInput/SayAllInput.inf");
const NAMES: [&str; 3] = ["SayAllInput.inf", "SayAllInput.sys", "sayallinput.cat"];
static HELPER_AUDIT: AtomicBool = AtomicBool::new(false);
#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegisterEventSourceW(server: *const u16, source: *const u16) -> *mut std::ffi::c_void;
    fn ReportEventW(
        handle: *mut std::ffi::c_void,
        kind: u16,
        category: u16,
        event: u32,
        sid: *const std::ffi::c_void,
        count: u16,
        bytes: u32,
        strings: *const *const u16,
        data: *const std::ffi::c_void,
    ) -> i32;
    fn DeregisterEventSource(handle: *mut std::ffi::c_void) -> i32;
}
fn event_log(message: &str) -> bool {
    let text: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let handle = RegisterEventSourceW(std::ptr::null(), w!("SayAllInput").as_ptr());
        if handle.is_null() {
            return false;
        }
        let ok = ReportEventW(
            handle,
            4,
            0,
            1001,
            std::ptr::null(),
            1,
            0,
            &text.as_ptr(),
            std::ptr::null(),
        );
        DeregisterEventSource(handle);
        ok != 0
    }
}
pub(super) fn enable_helper_audit() -> bool {
    HELPER_AUDIT.store(true, Ordering::Release);
    event_log("input_maintenance phase=helper_started protocol=3")
}
pub(super) fn note(message: String) {
    crate::gatt_note(message.clone());
    if HELPER_AUDIT.load(Ordering::Acquire) && !event_log(&message) {
        eprintln!("input_maintenance phase=audit_write_failed");
    }
}
struct Package {
    root: PathBuf,
    locks: Vec<File>,
}
struct MaintenanceLock(HANDLE);
impl MaintenanceLock {
    fn acquire() -> Result<Self, ComponentReason> {
        use windows::Win32::System::Threading::{CreateMutexW, WaitForSingleObject};
        let handle = unsafe { CreateMutexW(None, false, w!("Global\\SayAllInputMaintenance")) }
            .map_err(|_| ComponentReason::OperationInProgress)?;
        let wait = unsafe { WaitForSingleObject(handle, 0) };
        if wait != windows::Win32::Foundation::WAIT_OBJECT_0 && wait.0 != 0x80 {
            unsafe {
                let _ = windows::Win32::Foundation::CloseHandle(handle);
            }
            return Err(ComponentReason::OperationInProgress);
        }
        Ok(Self(handle))
    }
}
impl Drop for MaintenanceLock {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::System::Threading::ReleaseMutex(self.0);
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn root() -> Result<PathBuf, ComponentReason> {
    Ok(std::env::current_exe()
        .map_err(|_| ComponentReason::PathRejected)?
        .parent()
        .ok_or(ComponentReason::PathRejected)?
        .join("SayAllInput"))
}
fn verify(root: &Path) -> Result<Package, ComponentReason> {
    let root = root
        .canonicalize()
        .map_err(|_| ComponentReason::PackageMissing)?;
    // Anchor every resolved directory against rename/reparse replacement for
    // the lifetime of all path-based Windows trust/install calls.
    let mut directory_locks = Vec::new();
    for directory in root.ancestors().collect::<Vec<_>>().into_iter().rev() {
        let handle = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .custom_flags(0x02000000 | 0x00200000)
            .open(directory)
            .map_err(|_| ComponentReason::PathRejected)?;
        if handle
            .metadata()
            .map_err(|_| ComponentReason::PathRejected)?
            .file_attributes()
            & 0x400
            != 0
        {
            return Err(ComponentReason::PathRejected);
        }
        directory_locks.push(handle);
    }
    let mut locks = Vec::new();
    for (index, name) in NAMES.iter().enumerate() {
        let path = confined_path(&root, name)?;
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .custom_flags(0x00200000)
            .open(&path)
            .map_err(|_| ComponentReason::PackageMissing)?;
        if file
            .metadata()
            .map_err(|_| ComponentReason::PathRejected)?
            .file_attributes()
            & 0x400
            != 0
        {
            return Err(ComponentReason::PathRejected);
        }
        let size = file
            .metadata()
            .map_err(|_| ComponentReason::InvalidPackage)?
            .len();
        if size == 0 || size > 8 * 1024 * 1024 {
            return Err(ComponentReason::InvalidPackage);
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|_| ComponentReason::InvalidPackage)?;
        if (index == 0 && bytes != INF)
            || (index == 1 && hex(&Sha256::digest(&bytes)) != INPUT_SYS_SHA256)
        {
            return Err(ComponentReason::HashMismatch);
        }
        file.rewind().map_err(|_| ComponentReason::InvalidPackage)?;
        locks.push(file);
    }
    // The WHQL driver policy is stricter than ordinary file Authenticode.
    // Verify BOTH fixed members against THIS locked catalog; no signer-name test.
    for i in 0..2 {
        verify_member(&root.join(NAMES[2]), &root.join(NAMES[i]), &locks[i])?;
    }
    locks.extend(directory_locks);
    Ok(Package { root, locks })
}
fn verify_member(catalog: &Path, member: &Path, file: &File) -> Result<(), ComponentReason> {
    let mut admin = 0;
    unsafe {
        CryptCATAdminAcquireContext2(
            &mut admin,
            Some(&DRIVER_ACTION_VERIFY),
            w!("SHA256"),
            None,
            None,
        )
    }
    .map_err(|_| ComponentReason::SignatureInvalid)?;
    struct Admin(isize);
    impl Drop for Admin {
        fn drop(&mut self) {
            unsafe {
                let _ = CryptCATAdminReleaseContext(self.0, 0);
            }
        }
    }
    let _admin = Admin(admin);
    let mut hash = [0u8; 32];
    let mut length = 32;
    unsafe {
        CryptCATAdminCalcHashFromFileHandle2(
            admin,
            HANDLE(file.as_raw_handle()),
            &mut length,
            Some(hash.as_mut_ptr()),
            None,
        )
    }
    .map_err(|_| ComponentReason::SignatureInvalid)?;
    if length != 32 {
        return Err(ComponentReason::SignatureInvalid);
    }
    let tag: Vec<u16> = hex(&hash)
        .to_uppercase()
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let catalog = wide(catalog);
    let member = wide(member);
    let mut info = WINTRUST_CATALOG_INFO {
        cbStruct: std::mem::size_of::<WINTRUST_CATALOG_INFO>() as u32,
        pcwszCatalogFilePath: PCWSTR(catalog.as_ptr()),
        pcwszMemberTag: PCWSTR(tag.as_ptr()),
        pcwszMemberFilePath: PCWSTR(member.as_ptr()),
        hMemberFile: HANDLE(file.as_raw_handle()),
        pbCalculatedFileHash: hash.as_mut_ptr(),
        cbCalculatedFileHash: length,
        hCatAdmin: admin,
        ..Default::default()
    };
    let mut data = WINTRUST_DATA {
        cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
        dwUIChoice: WTD_UI_NONE,
        fdwRevocationChecks: WTD_REVOKE_WHOLECHAIN,
        dwUnionChoice: WTD_CHOICE_CATALOG,
        dwStateAction: WTD_STATEACTION_VERIFY,
        Anonymous: WINTRUST_DATA_0 {
            pCatalog: &mut info,
        },
        ..Default::default()
    };
    let mut action = DRIVER_ACTION_VERIFY;
    let code = unsafe {
        WinVerifyTrust(
            HWND::default(),
            &mut action,
            (&mut data as *mut WINTRUST_DATA).cast(),
        )
    };
    data.dwStateAction = WTD_STATEACTION_CLOSE;
    unsafe {
        WinVerifyTrust(
            HWND::default(),
            &mut action,
            (&mut data as *mut WINTRUST_DATA).cast(),
        );
    }
    if code != 0 {
        note(format!("input_maintenance phase=kernel_catalog_verify terminal_result=rejected windows_status={code}"));
        return Err(ComponentReason::SignatureInvalid);
    }
    Ok(())
}
#[link(name = "setupapi")]
unsafe extern "system" {
    fn SetupGetInfDriverStoreLocationW(
        inf: *const u16,
        platform: *const std::ffi::c_void,
        locale: *const u16,
        out: *mut u16,
        size: u32,
        needed: *mut u32,
    ) -> i32;
}
#[link(name = "newdev")]
unsafe extern "system" {
    fn DiInstallDriverW(
        hwnd: *mut std::ffi::c_void,
        inf: *const u16,
        flags: u32,
        reboot: *mut i32,
    ) -> i32;
    fn DiUninstallDriverW(
        hwnd: *mut std::ffi::c_void,
        inf: *const u16,
        flags: u32,
        reboot: *mut i32,
    ) -> i32;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetWindowsDirectoryW(buffer: *mut u16, size: u32) -> u32;
}
fn published_inf(directory: &Path) -> Result<Option<PathBuf>, ComponentReason> {
    let mut found = None;
    for entry in std::fs::read_dir(directory).map_err(|_| ComponentReason::DetectionFailed)? {
        let entry = entry.map_err(|_| ComponentReason::DetectionFailed)?;
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        let Some(number) = name
            .strip_prefix("oem")
            .and_then(|s| s.strip_suffix(".inf"))
        else {
            continue;
        };
        if number.is_empty() || !number.bytes().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let metadata = entry
            .metadata()
            .map_err(|_| ComponentReason::DetectionFailed)?;
        if metadata.len() != INF.len() as u64 {
            continue;
        }
        let bytes = std::fs::read(entry.path()).map_err(|_| ComponentReason::DetectionFailed)?;
        if bytes == INF {
            if found.is_some() {
                return Err(ComponentReason::IdentityUnavailable);
            }
            found = Some(entry.path());
        }
    }
    Ok(found)
}
fn stored_inf(_package: &Package) -> Result<Option<PathBuf>, ComponentReason> {
    let mut windows = vec![0u16; 32768];
    let length =
        unsafe { GetWindowsDirectoryW(windows.as_mut_ptr(), windows.len() as u32) } as usize;
    if length == 0 || length >= windows.len() {
        return Err(ComponentReason::DetectionFailed);
    }
    let directory = PathBuf::from(
        String::from_utf16(&windows[..length]).map_err(|_| ComponentReason::PathRejected)?,
    )
    .join("INF");
    // This API does not search by source content. Resolve an actual published
    // OEM INF first; only a successful complete scan proves exact-package absence.
    let Some(published) = published_inf(&directory)? else {
        return Ok(None);
    };
    let inf = wide(&published);
    let mut out = vec![0u16; 32768];
    let mut needed = 0;
    let ok = unsafe {
        SetupGetInfDriverStoreLocationW(
            inf.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            out.as_mut_ptr(),
            out.len() as u32,
            &mut needed,
        )
    };
    if ok == 0 {
        return Err(ComponentReason::DetectionFailed);
    }
    let n = out
        .iter()
        .position(|c| *c == 0)
        .ok_or(ComponentReason::PathRejected)?;
    let path =
        PathBuf::from(String::from_utf16(&out[..n]).map_err(|_| ComponentReason::PathRejected)?);
    // Never operate on an OEM package selected only by service/filename.
    let _verified = verify(path.parent().ok_or(ComponentReason::PathRejected)?)?;
    Ok(Some(path))
}
pub(super) fn inspect() -> ComponentStatus {
    let mut status = describe(
        ComponentKind::HidEnhancement,
        Evidence {
            installed: None,
            loaded: None,
            endpoints: None,
            restart: false,
            failure: None,
        },
    );
    let result = root().and_then(|p| verify(&p));
    match result {
        Err(reason) => {
            status.reason = reason;
            status.blockers = vec![reason];
            status.package = match reason {
                ComponentReason::PackageMissing | ComponentReason::PathRejected => {
                    PackageState::Missing
                }
                ComponentReason::SignatureInvalid => PackageState::SignatureMissing,
                _ => PackageState::Failed,
            };
        }
        Ok(package) => {
            status.package = PackageState::Trusted;
            match stored_inf(&package) {
                Ok(installed) => {
                    let present = installed.is_some();
                    let channel = crate::input_driver::inspect_channels();
                    let bound = channel.as_ref().map(|(n, v)| *n > 0 && *v).ok();
                    status.service_installed = if bound == Some(true) {
                        Some(true)
                    } else {
                        None
                    };
                    status.loaded = bound;
                    status.bound = bound;
                    status.installation = if !present {
                        InstallationState::NotInstalled
                    } else if bound == Some(true) {
                        InstallationState::Available
                    } else {
                        InstallationState::InstalledNotLoaded
                    };
                    status.reason = if !present {
                        ComponentReason::NotInstalled
                    } else if bound == Some(true) {
                        ComponentReason::Ready
                    } else {
                        ComponentReason::ServiceNotLoaded
                    };
                    status.blockers.clear();
                    status.allowed_actions = if present {
                        vec![ComponentAction::Repair, ComponentAction::Remove]
                    } else {
                        vec![ComponentAction::Install]
                    };
                }
                Err(reason) => {
                    status.reason = reason;
                    status.blockers = vec![reason];
                }
            }
        }
    }
    note(format!(
        "input_maintenance phase=inspect installation={:?} package={:?} reason={:?} actions={}",
        status.installation,
        status.package,
        status.reason,
        status.allowed_actions.len()
    ));
    status
}
pub(super) fn launch(action: ComponentAction) -> Result<Option<u32>, ComponentReason> {
    if !matches!(
        action,
        ComponentAction::Install | ComponentAction::Repair | ComponentAction::Remove
    ) {
        return Err(ComponentReason::OperationUnsupported);
    }
    let _pause = crate::input_driver::pause().map_err(|_| ComponentReason::OperationInProgress)?;
    let package = verify(&root()?)?;
    let guard = OperationGuard::acquire()?;
    let exe = std::env::current_exe().map_err(|_| ComponentReason::HelperUnavailable)?;
    let path = confined_path(
        exe.parent().ok_or(ComponentReason::PathRejected)?,
        "sayall-component-helper.exe",
    )?;
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&path)
        .map_err(|_| ComponentReason::HelperUnavailable)?;
    let expected =
        option_env!("SAYALL_COMPONENT_HELPER_SHA256").ok_or(ComponentReason::HelperUnavailable)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|_| ComponentReason::HelperUnavailable)?;
    if hex(&Sha256::digest(bytes)) != expected {
        return Err(ComponentReason::HashMismatch);
    }
    native::run_elevated_helper(
        VerifiedArtifact {
            path,
            _read_lock: file,
            _payload_locks: package.locks,
            _temporary_directory: None,
        },
        ComponentKind::HidEnhancement,
        action,
        guard,
    )
}
fn failed_maintenance(
    reboot: bool,
    reason: ComponentReason,
) -> Result<Option<u32>, ComponentReason> {
    if reboot {
        Ok(Some(3011))
    } else {
        Err(reason)
    }
}
#[derive(Debug, PartialEq, Eq)]
enum RecoveryChange {
    Preserve,
    RestoreMissing,
    RemoveNew,
}
fn recovery_change(before: bool, after: bool) -> RecoveryChange {
    match (before, after) {
        (true, false) => RecoveryChange::RestoreMissing,
        (false, true) => RecoveryChange::RemoveNew,
        _ => RecoveryChange::Preserve,
    }
}
fn rollback_new(
    package: &Package,
    originally_present: bool,
    mut reboot: bool,
) -> Result<Option<u32>, ComponentReason> {
    let current = stored_inf(package)?;
    let change = recovery_change(originally_present, current.is_some());
    if change == RecoveryChange::Preserve {
        note(format!("input_maintenance phase=recovery action=preserved exact_package_present={} original_operation=failed",current.is_some()));
        return failed_maintenance(reboot, ComponentReason::VerificationFailed);
    }
    // A new client may have opened after the first API ended. Re-establish
    // quiescence before a recovery mutation; never displace that client.
    let _channels = match crate::input_driver::maintenance_channels() {
        Ok(channels) => channels,
        Err(reason) => {
            note(format!(
                "input_maintenance phase=recovery_guard terminal_result=blocked reason={reason}"
            ));
            return failed_maintenance(reboot, ComponentReason::OperationInProgress);
        }
    };
    if change == RecoveryChange::RestoreMissing {
        // DiInstallDriver may remove a preinstalled identical package before
        // failing to add it again. Recheck, then restore from our still-locked
        // independently reverified source; presence alone is not binding proof.
        let source = verify(&package.root)?;
        let path = wide(&source.root.join(NAMES[0]));
        let mut requested_reboot = 0;
        let ok = unsafe {
            DiInstallDriverW(
                std::ptr::null_mut(),
                path.as_ptr(),
                0,
                &mut requested_reboot,
            )
        };
        let error = if ok == 0 {
            unsafe { GetLastError().0 }
        } else {
            0
        };
        reboot |= requested_reboot != 0;
        note(format!("input_maintenance phase=restore_previous exact_package_was_present=true present_after_failure=false success={} windows_error={error} reboot_required={reboot}",ok!=0));
        if ok == 0 {
            return failed_maintenance(reboot, ComponentReason::HelperFailed);
        }
        if !reboot && stored_inf(package)?.is_none() {
            return Err(ComponentReason::VerificationFailed);
        }
    } else {
        if let Some(staged) = current {
            let staged = wide(&staged);
            let mut requested_reboot = 0;
            let ok = unsafe {
                DiUninstallDriverW(
                    std::ptr::null_mut(),
                    staged.as_ptr(),
                    0,
                    &mut requested_reboot,
                )
            };
            let error = if ok == 0 {
                unsafe { GetLastError().0 }
            } else {
                0
            };
            reboot |= requested_reboot != 0;
            note(format!("input_maintenance phase=rollback success={} windows_error={error} reboot_required={reboot} prior_exact_package=false",ok!=0));
            if ok == 0 {
                return failed_maintenance(reboot, ComponentReason::HelperFailed);
            }
            if !reboot && stored_inf(package)?.is_some() {
                return Err(ComponentReason::VerificationFailed);
            }
        }
    }
    // Rollback is recovery from a failed operation, never installation success.
    failed_maintenance(reboot, ComponentReason::VerificationFailed)
}
pub(super) fn maintain(action: ComponentAction) -> Result<Option<u32>, ComponentReason> {
    if !matches!(
        action,
        ComponentAction::Install | ComponentAction::Repair | ComponentAction::Remove
    ) {
        return Err(ComponentReason::OperationUnsupported);
    }
    let _guard = OperationGuard::acquire()?;
    let package = verify(&root()?)?; // independently repeat inside the elevated process
    let _machine_lock = MaintenanceLock::acquire()?;
    let prior = stored_inf(&package)?;
    let channels = crate::input_driver::maintenance_channels().map_err(|reason| {
        note(format!(
            "input_maintenance phase=release_guard terminal_result=blocked reason={reason}"
        ));
        ComponentReason::OperationInProgress
    })?;
    if action == ComponentAction::Remove && prior.is_none() {
        return Ok(Some(0));
    }
    let path = if action == ComponentAction::Remove {
        prior.as_ref().unwrap().clone()
    } else {
        package.root.join(NAMES[0])
    };
    let inf = wide(&path);
    let mut reboot = 0;
    note(format!(
        "input_maintenance phase=windows_api action={action:?} package=fixed_sayall_input flags=0"
    ));
    let ok = unsafe {
        if action == ComponentAction::Remove {
            DiUninstallDriverW(std::ptr::null_mut(), inf.as_ptr(), 0, &mut reboot)
        } else {
            DiInstallDriverW(std::ptr::null_mut(), inf.as_ptr(), 0, &mut reboot)
        }
    };
    let error = if ok == 0 {
        unsafe { GetLastError().0 }
    } else {
        0
    };
    note(format!("input_maintenance phase=windows_api_completed action={action:?} success={} windows_error={error} reboot_required={}",ok!=0,reboot!=0));
    // The device-changing API is finished. Release our exclusive handles before
    // the read-only postcondition reopens interfaces; main remains paused.
    drop(channels);
    if ok == 0 {
        if action != ComponentAction::Remove {
            return rollback_new(&package, prior.is_some(), reboot != 0);
        }
        return failed_maintenance(
            reboot != 0,
            if error == 5 {
                ComponentReason::AccessDenied
            } else {
                ComponentReason::HelperFailed
            },
        );
    }
    if reboot != 0 {
        return Ok(Some(3010));
    }
    let postcondition: Result<bool, ComponentReason> = (|| {
        let after = stored_inf(&package)?;
        if action == ComponentAction::Remove {
            return Ok(after.is_none());
        }
        let (count, valid) = crate::input_driver::inspect_channels()
            .map_err(|_| ComponentReason::DetectionFailed)?;
        Ok(after.is_some() && count > 0 && valid)
    })();
    if postcondition != Ok(true) {
        note(format!(
            "input_maintenance phase=postcondition terminal_result=failed action={action:?}"
        ));
        if action != ComponentAction::Remove {
            return rollback_new(&package, prior.is_some(), false);
        }
        return Err(ComponentReason::VerificationFailed);
    }
    Ok(Some(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn input_package_recovery_changes_only_a_proven_delta() {
        assert_eq!(recovery_change(true, true), RecoveryChange::Preserve);
        assert_eq!(recovery_change(false, false), RecoveryChange::Preserve);
        assert_eq!(recovery_change(true, false), RecoveryChange::RestoreMissing);
        assert_eq!(recovery_change(false, true), RecoveryChange::RemoveNew);
    }
    #[test]
    fn input_package_exclusive_guard_must_close_before_postcondition_reopen() {
        let base = std::env::temp_dir().join(format!("sayall-share-test-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let _cleanup = WizardDirectory(base.clone());
        let path = base.join("channel");
        std::fs::write(&path, b"fixture").unwrap();
        let guard = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&path)
            .unwrap();
        assert!(std::fs::OpenOptions::new()
            .read(true)
            .share_mode(3)
            .open(&path)
            .is_err());
        drop(guard);
        assert!(std::fs::OpenOptions::new()
            .read(true)
            .share_mode(3)
            .open(&path)
            .is_ok());
    }
    #[test]
    fn input_package_failure_reboot_never_maps_to_success() {
        assert_eq!(
            failed_maintenance(false, ComponentReason::HelperFailed),
            Err(ComponentReason::HelperFailed)
        );
        assert_eq!(
            failed_maintenance(true, ComponentReason::HelperFailed),
            Ok(Some(3011))
        );
        let mut status = describe(
            ComponentKind::HidEnhancement,
            Evidence {
                installed: None,
                loaded: None,
                endpoints: None,
                restart: false,
                failure: None,
            },
        );
        status.restart_required = true;
        let (outcome, reason) = outcome_after_helper(Some(3011), ComponentAction::Install, &status);
        assert_eq!(outcome, OperationOutcome::Failed);
        let operation = ComponentOperation {
            component: ComponentKind::HidEnhancement,
            action: ComponentAction::Install,
            outcome,
            reason,
            status,
        };
        assert_eq!(operation.exit_code(), 3011);
    }
    #[test]
    fn input_package_published_selector_proves_absence_and_rejects_ambiguity() {
        let base =
            std::env::temp_dir().join(format!("sayall-published-test-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let _cleanup = WizardDirectory(base.clone());
        assert_eq!(published_inf(&base).unwrap(), None);
        std::fs::write(base.join("oem1.inf"), INF).unwrap();
        std::fs::write(base.join("oemword.inf"), INF).unwrap();
        assert_eq!(published_inf(&base).unwrap(), Some(base.join("oem1.inf")));
        std::fs::write(base.join("oem2.inf"), INF).unwrap();
        assert_eq!(
            published_inf(&base).unwrap_err(),
            ComponentReason::IdentityUnavailable
        );
        assert!(published_inf(&base.join("absent")).is_err());
    }
    #[test]
    fn input_package_missing_and_tampered_never_reach_install() {
        let base =
            std::env::temp_dir().join(format!("sayall-input-package-test-{}", std::process::id()));
        assert!(verify(&base).is_err());
        std::fs::create_dir_all(&base).unwrap();
        let _cleanup = WizardDirectory(base.clone());
        std::fs::write(base.join(NAMES[0]), b"untrusted inf").unwrap();
        assert!(matches!(verify(&base), Err(ComponentReason::HashMismatch)));
    }
    #[test]
    fn input_package_unsigned_candidate_is_rejected_read_only() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/input-driver");
        if !base.join(NAMES[2]).exists() {
            return;
        }
        assert!(matches!(
            verify(&base),
            Err(ComponentReason::SignatureInvalid)
        ));
    }
}

//! Read-only component evidence and the fixed, fail-closed elevation boundary.
//! Release trust is compiled into the application/helper, never supplied by IPC
//! or by an adjacent, editable manifest. VB-CABLE uses its official interactive
//! setup as the separate elevation boundary; no HID enhancement is implemented.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentKind {
    HidEnhancement,
    VbCable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentAction {
    OpenVendorWizard,
    Install,
    Repair,
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallationState {
    NotImplemented,
    Unknown,
    NotInstalled,
    InstalledNotLoaded,
    Available,
    RestartRequired,
    Incompatible,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageState {
    DownloadAvailable,
    Missing,
    Trusted,
    SignatureMissing,
    AuthorizationMissing,
    Incompatible,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentReason {
    NotImplemented,
    OfficialWizardRequired,
    DownloadFailed,
    WizardClosed,
    Ready,
    NotInstalled,
    ServiceNotLoaded,
    AudioEndpointsMissing,
    IdentityUnavailable,
    PackageMissing,
    SigningPolicyMissing,
    AuthorizationMissing,
    UninstallPackageMissing,
    UnsupportedPlatform,
    UnsupportedArchitecture,
    DetectionFailed,
    AccessDenied,
    RestartRequired,
    PathRejected,
    HashMismatch,
    SignatureInvalid,
    PublisherMismatch,
    VersionMismatch,
    InvalidPackage,
    UacCancelled,
    UacDenied,
    HelperUnavailable,
    HelperTimedOut,
    HelperFailed,
    VerificationFailed,
    OperationUnsupported,
    OperationInProgress,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentStatus {
    pub component: ComponentKind,
    pub installation: InstallationState,
    pub package: PackageState,
    pub installed_version: Option<String>,
    pub service_installed: Option<bool>,
    pub loaded: Option<bool>,
    pub bound: Option<bool>,
    pub audio_endpoints_ready: Option<bool>,
    pub restart_required: bool,
    pub reason: ComponentReason,
    pub blockers: Vec<ComponentReason>,
    pub allowed_actions: Vec<ComponentAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationOutcome {
    WizardClosed,
    Blocked,
    Cancelled,
    Denied,
    TimedOut,
    RestartRequired,
    Failed,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentOperation {
    pub component: ComponentKind,
    pub action: ComponentAction,
    pub outcome: OperationOutcome,
    pub reason: ComponentReason,
    pub status: ComponentStatus,
}

impl ComponentOperation {
    pub fn exit_code(&self) -> i32 {
        match self.outcome {
            OperationOutcome::Completed | OperationOutcome::WizardClosed => 0,
            OperationOutcome::Blocked => 50,
            OperationOutcome::Cancelled => 1223,
            OperationOutcome::Denied => 5,
            OperationOutcome::TimedOut => 1460,
            OperationOutcome::RestartRequired => 3010,
            OperationOutcome::Failed => 1,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Evidence {
    installed: Option<bool>,
    loaded: Option<bool>,
    endpoints: Option<bool>,
    restart: bool,
    failure: Option<ComponentReason>,
}

fn describe(component: ComponentKind, evidence: Evidence) -> ComponentStatus {
    let (installation, reason) = if let Some(reason) = evidence.failure {
        (
            if reason == ComponentReason::UnsupportedPlatform {
                InstallationState::Incompatible
            } else {
                InstallationState::Failed
            },
            reason,
        )
    } else if evidence.restart {
        (
            InstallationState::RestartRequired,
            ComponentReason::RestartRequired,
        )
    } else if evidence.installed == Some(false) {
        (
            InstallationState::NotInstalled,
            ComponentReason::NotInstalled,
        )
    } else if evidence.installed.is_none() {
        (
            InstallationState::Unknown,
            ComponentReason::IdentityUnavailable,
        )
    } else if evidence.loaded != Some(true) {
        (
            InstallationState::InstalledNotLoaded,
            ComponentReason::ServiceNotLoaded,
        )
    } else if evidence.endpoints != Some(true) {
        (
            InstallationState::InstalledNotLoaded,
            ComponentReason::AudioEndpointsMissing,
        )
    } else {
        (InstallationState::Available, ComponentReason::Ready)
    };
    let wizard_supported = cfg!(all(windows, target_arch = "x86_64"));
    let is_vb = component == ComponentKind::VbCable;
    let blockers = if is_vb {
        vec![ComponentReason::OfficialWizardRequired]
    } else {
        vec![ComponentReason::NotImplemented]
    };
    ComponentStatus {
        component,
        installation: if is_vb {
            installation
        } else {
            InstallationState::NotImplemented
        },
        package: if is_vb {
            PackageState::DownloadAvailable
        } else {
            PackageState::Missing
        },
        installed_version: None,
        service_installed: evidence.installed,
        loaded: evidence.loaded,
        bound: None,
        audio_endpoints_ready: evidence.endpoints,
        restart_required: evidence.restart,
        reason: if is_vb {
            reason
        } else {
            ComponentReason::NotImplemented
        },
        blockers,
        allowed_actions: if is_vb && wizard_supported {
            vec![ComponentAction::OpenVendorWizard]
        } else {
            Vec::new()
        },
    }
}

fn inspect_one(component: ComponentKind) -> ComponentStatus {
    let evidence = match component {
        // A guessed service name is not evidence of this product's filter.
        ComponentKind::HidEnhancement => Evidence {
            installed: None,
            loaded: None,
            endpoints: None,
            restart: false,
            failure: None,
        },
        ComponentKind::VbCable => probe_vb_cable(),
    };
    let status = describe(component, evidence);
    crate::gatt_note(format!("component_support action=inspect component={component:?} installation={:?} reason={:?} allowed_actions={}", status.installation, status.reason, status.allowed_actions.len()));
    status
}

pub fn inspect_components() -> Vec<ComponentStatus> {
    [ComponentKind::HidEnhancement, ComponentKind::VbCable]
        .into_iter()
        .map(inspect_one)
        .collect()
}

/// Fixed CLI vocabulary; paths, commands, extra switches and manifests are rejected.
pub fn parse_helper_arguments(arguments: &[String]) -> Option<(ComponentKind, ComponentAction)> {
    let [component, action] = arguments else {
        return None;
    };
    let component = match component.as_str() {
        "hid_enhancement" => ComponentKind::HidEnhancement,
        "vb_cable" => ComponentKind::VbCable,
        _ => return None,
    };
    let action = match action.as_str() {
        "install" => ComponentAction::Install,
        "repair" => ComponentAction::Repair,
        "remove" => ComponentAction::Remove,
        _ => return None,
    };
    Some((component, action))
}

// No generic installer strategy: only the independently verified vendor wizard
// exists. The separate product Helper cannot perform silent driver operations.
fn unsupported_operation_reason(component: ComponentKind) -> ComponentReason {
    match component {
        ComponentKind::VbCable => ComponentReason::OfficialWizardRequired,
        ComponentKind::HidEnhancement => ComponentReason::NotImplemented,
    }
}

/// Call on a background thread after the visible VB-Audio donationware notice
/// and explicit wizard confirmation. Legacy silent actions remain unsupported.
pub fn perform_component_action(
    component: ComponentKind,
    action: ComponentAction,
) -> ComponentOperation {
    if action == ComponentAction::OpenVendorWizard {
        return operation_result(component, action, open_vendor_wizard(component));
    }
    operation_result(
        component,
        action,
        Err(unsupported_operation_reason(component)),
    )
}

/// Directly invoking the product Helper never bypasses the vendor wizard.
pub fn helper_request(component: ComponentKind, action: ComponentAction) -> ComponentOperation {
    operation_result(
        component,
        action,
        Err(unsupported_operation_reason(component)),
    )
}

fn operation_result(
    component: ComponentKind,
    action: ComponentAction,
    result: Result<Option<u32>, ComponentReason>,
) -> ComponentOperation {
    let status = inspect_one(component);
    let (outcome, reason) = match result {
        Ok(exit) => outcome_after_helper(exit, action, &status),
        Err(reason) => (
            match reason {
                ComponentReason::UacCancelled => OperationOutcome::Cancelled,
                ComponentReason::UacDenied => OperationOutcome::Denied,
                ComponentReason::HelperFailed | ComponentReason::DownloadFailed => {
                    OperationOutcome::Failed
                }
                _ => OperationOutcome::Blocked,
            },
            reason,
        ),
    };
    crate::gatt_note(format!("component_support action={action:?} component={component:?} terminal_result={outcome:?} reason={reason:?}"));
    ComponentOperation {
        component,
        action,
        outcome,
        reason,
        status,
    }
}

static HELPER_RUNNING: AtomicBool = AtomicBool::new(false);
struct OperationGuard;
impl OperationGuard {
    fn acquire() -> Result<Self, ComponentReason> {
        HELPER_RUNNING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| Self)
            .map_err(|_| ComponentReason::OperationInProgress)
    }
}
impl Drop for OperationGuard {
    fn drop(&mut self) {
        HELPER_RUNNING.store(false, Ordering::Release);
    }
}

/// Installer exit status alone cannot prove a usable device. Used at the fixed
/// Helper boundary; even exit 0 must be followed by component-specific evidence.
fn outcome_after_helper(
    exit: Option<u32>,
    action: ComponentAction,
    status: &ComponentStatus,
) -> (OperationOutcome, ComponentReason) {
    match exit {
        Some(0) if action == ComponentAction::OpenVendorWizard => (
            OperationOutcome::WizardClosed,
            ComponentReason::WizardClosed,
        ),
        None => (OperationOutcome::TimedOut, ComponentReason::HelperTimedOut),
        Some(1223) => (OperationOutcome::Cancelled, ComponentReason::UacCancelled),
        Some(5) => (OperationOutcome::Denied, ComponentReason::UacDenied),
        Some(3010) | Some(1641) => (
            OperationOutcome::RestartRequired,
            ComponentReason::RestartRequired,
        ),
        Some(0) => {
            let verified = if action == ComponentAction::Remove {
                status.installation == InstallationState::NotInstalled
            } else {
                status.installation == InstallationState::Available
            };
            if verified {
                (OperationOutcome::Completed, ComponentReason::Ready)
            } else {
                (
                    OperationOutcome::Failed,
                    ComponentReason::VerificationFailed,
                )
            }
        }
        Some(_) => (OperationOutcome::Failed, ComponentReason::HelperFailed),
    }
}

/// Release-bound metadata. This type is deliberately private and is not a
/// deserializable IPC or disk format. The current release supplies no instances.
#[allow(dead_code)]
struct TrustedArtifact {
    relative_path: &'static str,
    sha256: [u8; 32],
    machine: u16,
    file_version: [u16; 4],
    publisher: &'static str,
    signer_sha256: [u8; 32],
}

#[allow(dead_code)]
struct VerifiedArtifact {
    path: PathBuf,
    _read_lock: File,
    _payload_locks: Vec<File>,
    _temporary_directory: Option<WizardDirectory>,
}

// Obtained from the official product-page link and independently checked on
// 2026-09-11. These identify this exact Pack45, not arbitrary future downloads.
const VB_DOWNLOAD: &str = "https://download.vb-audio.com/Download_CABLE/VBCABLE_Driver_Pack45.zip";
const VB_ZIP_SHA256: [u8; 32] = [
    0xB9, 0x50, 0xE3, 0x9F, 0x01, 0xAF, 0x1D, 0x04, 0xEA, 0x62, 0x3C, 0x8F, 0x6D, 0x8E, 0xB9, 0xB6,
    0xEA, 0x5C, 0x47, 0x7C, 0x63, 0x72, 0x95, 0xFA, 0xBF, 0x20, 0x63, 0x1C, 0x85, 0x11, 0x6B, 0xFB,
];
static VB_SETUP: TrustedArtifact = TrustedArtifact {
    relative_path: "VBCABLE_Setup_x64.exe",
    sha256: [
        0x73, 0x4C, 0x35, 0xDF, 0xA6, 0xD9, 0x8F, 0x48, 0x78, 0x2A, 0x45, 0x16, 0x33, 0xCE, 0xB4,
        0x71, 0x16, 0x6E, 0xC7, 0x0D, 0x60, 0x48, 0x2F, 0xD8, 0x9A, 0x11, 0x23, 0xD0, 0xEE, 0x3C,
        0x4F, 0x41,
    ],
    machine: 0x8664,
    file_version: [2, 1, 5, 8],
    publisher: "BUREL VINCENT Entrepreneur individuel",
    signer_sha256: [
        0x6C, 0x37, 0xD5, 0xDD, 0xA6, 0xB7, 0xE8, 0x80, 0xB3, 0x83, 0x39, 0xE2, 0x33, 0x68, 0x9C,
        0x30, 0x2E, 0x7D, 0x26, 0xCC, 0xFD, 0xAC, 0x01, 0x9B, 0x3C, 0x3F, 0x4B, 0xD1, 0x63, 0x30,
        0xA7, 0x94,
    ],
};

/// Own only the newly-created task directory. Failures go to the Recycle Bin;
/// never fall back to permanent deletion or touch the user's own downloads.
struct WizardDirectory(PathBuf);
impl Drop for WizardDirectory {
    fn drop(&mut self) {
        #[cfg(windows)]
        native::recycle_wizard_directory(&self.0);
    }
}

fn unpack_vendor_zip(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, ComponentReason> {
    if bytes.len() > 4 * 1024 * 1024 || <[u8; 32]>::from(Sha256::digest(bytes)) != VB_ZIP_SHA256 {
        return Err(ComponentReason::HashMismatch);
    }
    decode_vendor_zip(bytes)
}

fn decode_vendor_zip(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, ComponentReason> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|_| ComponentReason::InvalidPackage)?;
    if archive.is_empty() || archive.len() > 64 {
        return Err(ComponentReason::InvalidPackage);
    }
    let mut entries = Vec::new();
    let mut names = std::collections::HashSet::new();
    let mut total = 0u64;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|_| ComponentReason::InvalidPackage)?;
        let name = entry.name().to_owned();
        // The official pack is flat. Reject paths, ADS, links and duplicate
        // names before creating anything on disk.
        if name.is_empty()
            || name
                .chars()
                .any(|c| !c.is_ascii_alphanumeric() && !matches!(c, '.' | '_' | '-'))
            || name.starts_with('.')
            || name.ends_with('.')
            || entry.is_dir()
            || entry.is_symlink()
            || !names.insert(name.to_ascii_lowercase())
        {
            return Err(ComponentReason::PathRejected);
        }
        total = total
            .checked_add(entry.size())
            .ok_or(ComponentReason::InvalidPackage)?;
        if total > 32 * 1024 * 1024 {
            return Err(ComponentReason::InvalidPackage);
        }
        let mut data = Vec::new();
        entry
            .take(32 * 1024 * 1024 + 1)
            .read_to_end(&mut data)
            .map_err(|_| ComponentReason::InvalidPackage)?;
        if data.len() > 32 * 1024 * 1024 {
            return Err(ComponentReason::InvalidPackage);
        }
        entries.push((name, data));
    }
    if !names.contains("vbcable_setup_x64.exe")
        || !names.contains("vbaudio_cable64_win10.sys")
        || !names.contains("vbaudio_cable64_win10.cat")
        || !names.contains("vbmmecable64_win10.inf")
    {
        return Err(ComponentReason::InvalidPackage);
    }
    Ok(entries)
}

#[cfg(windows)]
fn prepare_vendor_package(bytes: &[u8]) -> Result<VerifiedArtifact, ComponentReason> {
    use std::io::Write;
    use std::os::windows::fs::OpenOptionsExt;
    let entries = unpack_vendor_zip(bytes)?;
    let base = std::env::temp_dir()
        .canonicalize()
        .map_err(|_| ComponentReason::PathRejected)?;
    reject_reparse(&base)?;
    let directory = base.join(format!(
        "sayall-vbcable-{}",
        crate::templates::new_template_id()
    ));
    std::fs::create_dir(&directory).map_err(|_| ComponentReason::AccessDenied)?;
    let directory = WizardDirectory(directory);
    let mut locks = Vec::new();
    for (name, data) in entries {
        let path = directory.0.join(name);
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .share_mode(0)
            .open(&path)
            .map_err(|_| ComponentReason::AccessDenied)?;
        output
            .write_all(&data)
            .map_err(|_| ComponentReason::InvalidPackage)?;
        output
            .sync_all()
            .map_err(|_| ComponentReason::InvalidPackage)?;
        drop(output);
        reject_reparse(&path)?;
        let mut lock = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&path)
            .map_err(|_| ComponentReason::AccessDenied)?;
        let mut actual = Vec::new();
        std::io::Read::by_ref(&mut lock)
            .take(32 * 1024 * 1024 + 1)
            .read_to_end(&mut actual)
            .map_err(|_| ComponentReason::InvalidPackage)?;
        if actual != data {
            return Err(ComponentReason::HashMismatch);
        }
        locks.push(lock);
    }
    let mut artifact = verify_artifact(&directory.0, &VB_SETUP)?;
    artifact._payload_locks = locks;
    artifact._temporary_directory = Some(directory);
    crate::gatt_note("component_support action=verify_vendor_pack result=passed package=pack45 signature=trusted architecture=x64".to_owned());
    Ok(artifact)
}

#[cfg(windows)]
fn download_vendor_package() -> Result<VerifiedArtifact, ComponentReason> {
    crate::gatt_note(
        "component_support action=download component=VbCable source=official package=pack45"
            .to_owned(),
    );
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(std::time::Duration::from_secs(20))
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|_| ComponentReason::DownloadFailed)?;
    let response = client
        .get(VB_DOWNLOAD)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|_| ComponentReason::DownloadFailed)?;
    let mut bytes = Vec::new();
    response
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ComponentReason::DownloadFailed)?;
    prepare_vendor_package(&bytes)
}

fn open_vendor_wizard(component: ComponentKind) -> Result<Option<u32>, ComponentReason> {
    if component != ComponentKind::VbCable {
        return Err(ComponentReason::NotImplemented);
    }
    #[cfg(all(windows, target_arch = "x86_64"))]
    {
        let guard = OperationGuard::acquire()?;
        let package = download_vendor_package()?;
        native::run_elevated_helper(package, component, ComponentAction::OpenVendorWizard, guard)
    }
    #[cfg(not(all(windows, target_arch = "x86_64")))]
    Err(ComponentReason::UnsupportedArchitecture)
}

fn confined_path(root: &Path, relative: &str) -> Result<PathBuf, ComponentReason> {
    if relative
        .chars()
        .any(|ch| ch == ':' || ch == '\\' || ch.is_control())
        || relative
            .split('/')
            .any(|part| part.ends_with('.') || part.ends_with(' '))
    {
        return Err(ComponentReason::PathRejected);
    }
    let relative = Path::new(relative);
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(ComponentReason::PathRejected);
    }
    reject_reparse(root)?;
    let root = root
        .canonicalize()
        .map_err(|_| ComponentReason::PathRejected)?;
    let mut path = root.clone();
    for part in relative.components() {
        path.push(part);
        reject_reparse(&path)?;
    }
    let path = path
        .canonicalize()
        .map_err(|_| ComponentReason::PackageMissing)?;
    if !path.starts_with(root) {
        return Err(ComponentReason::PathRejected);
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(
        installed: Option<bool>,
        loaded: Option<bool>,
        endpoints: Option<bool>,
    ) -> Evidence {
        Evidence {
            installed,
            loaded,
            endpoints,
            restart: false,
            failure: None,
        }
    }

    fn fixture() -> PathBuf {
        let directory = std::env::temp_dir().join(crate::templates::new_template_id());
        std::fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn pe_artifact(bytes: &[u8], machine: u16) -> TrustedArtifact {
        TrustedArtifact {
            relative_path: "test.exe",
            sha256: Sha256::digest(bytes).into(),
            machine,
            file_version: [1, 0, 0, 0],
            publisher: "unit-test fixture, never trusted in production",
            signer_sha256: [0; 32],
        }
    }

    fn pe_bytes(machine: u16) -> Vec<u8> {
        let mut bytes = vec![0u8; 70];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[60..64].copy_from_slice(&64u32.to_le_bytes());
        bytes[64..68].copy_from_slice(b"PE\0\0");
        bytes[68..70].copy_from_slice(&machine.to_le_bytes());
        bytes
    }

    #[test]
    fn service_presence_alone_never_means_available() {
        for loaded in [None, Some(false), Some(true)] {
            let status = describe(
                ComponentKind::VbCable,
                evidence(Some(true), loaded, Some(false)),
            );
            assert_ne!(status.installation, InstallationState::Available);
            assert!(!status.restart_required);
        }
        let ready = describe(
            ComponentKind::VbCable,
            evidence(Some(true), Some(true), Some(true)),
        );
        assert_eq!(ready.installation, InstallationState::Available);
        assert_eq!(
            ready
                .allowed_actions
                .contains(&ComponentAction::OpenVendorWizard),
            cfg!(all(windows, target_arch = "x86_64"))
        );
        assert!(ready
            .blockers
            .contains(&ComponentReason::OfficialWizardRequired));
        assert_eq!(ready.bound, None);
        assert_eq!(ready.installed_version, None);
        assert_eq!(
            describe(ComponentKind::HidEnhancement, evidence(None, None, None)).installation,
            InstallationState::NotImplemented
        );
    }

    #[test]
    fn explicit_component_reboot_and_query_errors_are_preserved() {
        let mut observed = evidence(Some(true), Some(false), None);
        observed.restart = true;
        assert_eq!(
            describe(ComponentKind::VbCable, observed).installation,
            InstallationState::RestartRequired
        );
        observed.failure = Some(ComponentReason::AccessDenied);
        assert_eq!(
            describe(ComponentKind::VbCable, observed).reason,
            ComponentReason::AccessDenied
        );
    }

    #[test]
    fn helper_accepts_only_exact_component_action_pairs() {
        assert_eq!(
            parse_helper_arguments(&["vb_cable".into(), "install".into()]),
            Some((ComponentKind::VbCable, ComponentAction::Install))
        );
        for args in [
            vec![],
            vec!["vb_cable", "install", "--manifest", "other.json"],
            vec!["other", "install"],
            vec!["vb_cable", "cmd.exe"],
            vec!["../driver", "remove"],
        ] {
            assert!(parse_helper_arguments(
                &args.into_iter().map(str::to_owned).collect::<Vec<_>>()
            )
            .is_none());
        }
    }

    #[test]
    fn absent_release_material_blocks_every_write_without_elevation() {
        for component in [ComponentKind::HidEnhancement, ComponentKind::VbCable] {
            for action in [
                ComponentAction::Install,
                ComponentAction::Repair,
                ComponentAction::Remove,
            ] {
                assert_eq!(
                    helper_request(component, action).outcome,
                    OperationOutcome::Blocked
                );
            }
        }
        // A real exported entrypoint follows the same gate before it can open a
        // package or launch a process. Its status recheck is read-only.
        let result =
            perform_component_action(ComponentKind::HidEnhancement, ComponentAction::Install);
        assert_eq!(result.outcome, OperationOutcome::Blocked);
        assert_eq!(result.reason, ComponentReason::NotImplemented);
        assert_eq!(result.exit_code(), 50);
    }

    #[test]
    fn helper_exit_zero_requires_actual_postcondition_and_never_implies_reboot() {
        let absent = describe(
            ComponentKind::VbCable,
            evidence(Some(false), Some(false), Some(false)),
        );
        assert_eq!(
            outcome_after_helper(Some(0), ComponentAction::OpenVendorWizard, &absent),
            (
                OperationOutcome::WizardClosed,
                ComponentReason::WizardClosed
            )
        );
        assert_eq!(
            outcome_after_helper(Some(0), ComponentAction::Install, &absent).0,
            OperationOutcome::Failed
        );
        assert_eq!(
            outcome_after_helper(Some(0), ComponentAction::Remove, &absent).0,
            OperationOutcome::Completed
        );
        for (exit, outcome) in [
            (Some(1223), OperationOutcome::Cancelled),
            (Some(5), OperationOutcome::Denied),
            (None, OperationOutcome::TimedOut),
            (Some(3010), OperationOutcome::RestartRequired),
            (Some(9), OperationOutcome::Failed),
        ] {
            assert_eq!(
                outcome_after_helper(exit, ComponentAction::Repair, &absent).0,
                outcome
            );
        }
    }

    #[test]
    fn vendor_wizard_rejects_other_components_without_processes() {
        assert_eq!(
            open_vendor_wizard(ComponentKind::HidEnhancement),
            Err(ComponentReason::NotImplemented)
        );
        assert!(
            parse_helper_arguments(&["vb_cable".into(), "open_vendor_wizard".into()]).is_none()
        );
        assert_eq!(
            unpack_vendor_zip(b"untrusted downloaded bytes").unwrap_err(),
            ComponentReason::HashMismatch
        );
    }

    #[test]
    fn vendor_archive_rejects_traversal_duplicates_and_missing_payload() {
        use std::io::Write;
        for names in [
            vec!["../escape.exe"],
            vec!["C:stream"],
            vec!["same.inf", "SAME.inf"],
            vec!["readme.txt"],
        ] {
            let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
            for name in names {
                writer
                    .start_file(name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                writer.write_all(b"fixture").unwrap();
            }
            let bytes = writer.finish().unwrap().into_inner();
            assert!(decode_vendor_zip(&bytes).is_err());
        }
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires the independently downloaded official Pack45; verifies only, never launches"]
    fn official_vendor_pack_verifies_without_launching() {
        let path = std::env::var_os("SAYALL_VBCABLE_TEST_ZIP").expect("official test ZIP location");
        let bytes = std::fs::read(path).unwrap();
        let package = prepare_vendor_package(&bytes).unwrap();
        let directory = package.path.parent().unwrap().to_owned();
        assert!(package._payload_locks.len() > 20);
        assert_eq!(package.path.file_name().unwrap(), "VBCABLE_Setup_x64.exe");
        drop(package);
        assert!(
            !directory.exists(),
            "verified temporary payload must be recycled"
        );
        let mut changed = bytes;
        changed[0] ^= 1;
        assert_eq!(
            unpack_vendor_zip(&changed).unwrap_err(),
            ComponentReason::HashMismatch
        );
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "official HTTPS download and signature verification only; never launches"]
    fn official_vendor_download_verifies_without_launching() {
        let package = download_vendor_package().unwrap();
        let directory = package.path.parent().unwrap().to_owned();
        drop(package);
        assert!(
            !directory.exists(),
            "downloaded temporary payload must be recycled"
        );
    }

    #[test]
    fn package_path_rejects_escape_unc_ads_and_nonexistent_files() {
        let root = fixture();
        std::fs::write(root.join("safe.exe"), b"fixture").unwrap();
        assert!(confined_path(&root, "safe.exe").is_ok());
        for relative in [
            "../safe.exe",
            "/safe.exe",
            "C:\\safe.exe",
            "\\\\host\\safe.exe",
            "safe.exe:stream",
            "safe.exe.",
            "safe.exe ",
        ] {
            assert!(matches!(
                confined_path(&root, relative),
                Err(ComponentReason::PathRejected)
            ));
        }
        assert_eq!(
            confined_path(&root, "missing.exe").unwrap_err(),
            ComponentReason::PackageMissing
        );
    }

    #[test]
    fn package_hash_and_machine_validation_reject_tampering() {
        let root = fixture();
        let bytes = pe_bytes(0x8664);
        let path = root.join("test.exe");
        std::fs::write(&path, &bytes).unwrap();
        let valid = pe_artifact(&bytes, 0x8664);
        assert!(verify_bytes(&mut File::open(&path).unwrap(), &valid).is_ok());
        let wrong_machine = pe_artifact(&bytes, 0xaa64);
        assert_eq!(
            verify_bytes(&mut File::open(&path).unwrap(), &wrong_machine),
            Err(ComponentReason::UnsupportedArchitecture)
        );
        let mut tampered = bytes;
        tampered[5] = 1;
        std::fs::write(&path, tampered).unwrap();
        assert_eq!(
            verify_bytes(&mut File::open(path).unwrap(), &valid),
            Err(ComponentReason::HashMismatch)
        );
    }

    #[cfg(windows)]
    #[test]
    fn unsigned_package_is_rejected_by_real_windows_trust_api() {
        let root = fixture();
        let bytes = pe_bytes(0x8664);
        std::fs::write(root.join("test.exe"), &bytes).unwrap();
        let artifact = pe_artifact(&bytes, 0x8664);
        assert!(matches!(
            verify_artifact(&root, &artifact),
            Err(ComponentReason::SignatureInvalid)
        ));
    }

    #[test]
    fn operation_guard_prevents_overlap_until_owner_finishes() {
        let first = OperationGuard::acquire().unwrap();
        assert!(matches!(
            OperationGuard::acquire(),
            Err(ComponentReason::OperationInProgress)
        ));
        drop(first);
        assert!(OperationGuard::acquire().is_ok());
    }
}

fn reject_reparse(path: &Path) -> Result<(), ComponentReason> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| ComponentReason::PackageMissing)?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(ComponentReason::PathRejected);
        }
    }
    if metadata.file_type().is_symlink() {
        return Err(ComponentReason::PathRejected);
    }
    Ok(())
}

fn verify_bytes(file: &mut File, artifact: &TrustedArtifact) -> Result<(), ComponentReason> {
    let mut hasher = Sha256::new();
    let mut chunk = [0u8; 64 * 1024];
    loop {
        let length = file
            .read(&mut chunk)
            .map_err(|_| ComponentReason::InvalidPackage)?;
        if length == 0 {
            break;
        }
        hasher.update(&chunk[..length]);
    }
    let actual: [u8; 32] = hasher.finalize().into();
    if actual != artifact.sha256 {
        return Err(ComponentReason::HashMismatch);
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|_| ComponentReason::InvalidPackage)?;
    let mut dos = [0; 64];
    file.read_exact(&mut dos)
        .map_err(|_| ComponentReason::InvalidPackage)?;
    if &dos[..2] != b"MZ" {
        return Err(ComponentReason::InvalidPackage);
    }
    let offset = u32::from_le_bytes(dos[60..64].try_into().unwrap()) as u64;
    if offset < 64 || offset > 16 * 1024 * 1024 {
        return Err(ComponentReason::InvalidPackage);
    }
    file.seek(SeekFrom::Start(offset))
        .map_err(|_| ComponentReason::InvalidPackage)?;
    let mut header = [0; 6];
    file.read_exact(&mut header)
        .map_err(|_| ComponentReason::InvalidPackage)?;
    if &header[..4] != b"PE\0\0" {
        return Err(ComponentReason::InvalidPackage);
    }
    if u16::from_le_bytes([header[4], header[5]]) != artifact.machine {
        return Err(ComponentReason::UnsupportedArchitecture);
    }
    Ok(())
}

#[allow(dead_code)]
fn verify_artifact(
    root: &Path,
    artifact: &TrustedArtifact,
) -> Result<VerifiedArtifact, ComponentReason> {
    let native_machine = if cfg!(target_arch = "x86_64") {
        0x8664
    } else if cfg!(target_arch = "aarch64") {
        0xaa64
    } else {
        return Err(ComponentReason::UnsupportedArchitecture);
    };
    if artifact.machine != native_machine {
        return Err(ComponentReason::UnsupportedArchitecture);
    }
    let path = confined_path(root, artifact.relative_path)?;
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(1); // retain deny-write/delete lock through execution.
    }
    let mut file = options
        .open(&path)
        .map_err(|_| ComponentReason::PackageMissing)?;
    verify_bytes(&mut file, artifact)?;
    #[cfg(windows)]
    native::verify_signature_and_version(&path, &file, artifact)?;
    #[cfg(not(windows))]
    return Err(ComponentReason::UnsupportedPlatform);
    #[cfg(windows)]
    Ok(VerifiedArtifact {
        path,
        _read_lock: file,
        _payload_locks: Vec::new(),
        _temporary_directory: None,
    })
}

#[cfg(not(windows))]
fn probe_vb_cable() -> Evidence {
    Evidence {
        installed: None,
        loaded: None,
        endpoints: None,
        restart: false,
        failure: Some(ComponentReason::UnsupportedPlatform),
    }
}

#[cfg(windows)]
fn probe_vb_cable() -> Evidence {
    std::thread::spawn(native::probe_vb_cable)
        .join()
        .unwrap_or(Evidence {
            installed: None,
            loaded: None,
            endpoints: None,
            restart: false,
            failure: Some(ComponentReason::DetectionFailed),
        })
}

#[cfg(windows)]
mod native {
    use super::*;
    use std::os::windows::{ffi::OsStrExt, io::AsRawHandle};
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Services::*;

    struct ServiceHandle(SC_HANDLE);

    fn shell_path(path: &Path) -> Result<Vec<u16>, ComponentReason> {
        let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        if wide.starts_with(&[92, 92, 63, 92]) && wide.get(5) == Some(&58) {
            wide.drain(..4);
        }
        if wide.len() >= 260 || wide.get(1) != Some(&58) || wide.get(2) != Some(&92) {
            return Err(ComponentReason::PathRejected);
        }
        wide.push(0);
        Ok(wide)
    }

    pub(super) fn run_elevated_helper(
        helper: VerifiedArtifact,
        component: ComponentKind,
        action: ComponentAction,
        guard: OperationGuard,
    ) -> Result<Option<u32>, ComponentReason> {
        // ShellExecuteEx may require COM. Never assume a reused host worker's
        // apartment; the explicit wizard owns a dedicated initialized thread.
        std::thread::spawn(move || {
            use windows::Win32::System::Com::{
                CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED,
            };
            unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok() }
                .map_err(|_| ComponentReason::HelperFailed)?;
            struct Com;
            impl Drop for Com {
                fn drop(&mut self) {
                    unsafe {
                        CoUninitialize();
                    }
                }
            }
            let _com = Com;
            run_elevated_in_apartment(helper, component, action, guard)
        })
        .join()
        .map_err(|_| ComponentReason::HelperFailed)?
    }

    fn run_elevated_in_apartment(
        helper: VerifiedArtifact,
        component: ComponentKind,
        action: ComponentAction,
        guard: OperationGuard,
    ) -> Result<Option<u32>, ComponentReason> {
        use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0, WAIT_TIMEOUT};
        use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
        use windows::Win32::UI::Shell::{
            ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS,
            SHELLEXECUTEINFOW,
        };
        use windows::Win32::UI::WindowsAndMessaging::{SW_HIDE, SW_SHOWNORMAL};
        let vendor_wizard = action == ComponentAction::OpenVendorWizard;
        let executable = shell_path(&helper.path)?;
        let component = match component {
            ComponentKind::HidEnhancement => "hid_enhancement",
            ComponentKind::VbCable => "vb_cable",
        };
        let action = match action {
            ComponentAction::OpenVendorWizard => "",
            ComponentAction::Install => "install",
            ComponentAction::Repair => "repair",
            ComponentAction::Remove => "remove",
        };
        let arguments: Vec<u16> = if vendor_wizard {
            String::new()
        } else {
            format!("{component} {action}")
        }
        .encode_utf16()
        .chain(Some(0))
        .collect();
        let directory = shell_path(helper.path.parent().ok_or(ComponentReason::PathRejected)?)?;
        let mut launch = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_FLAG_NO_UI | SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
            lpVerb: w!("runas"),
            lpFile: PCWSTR(executable.as_ptr()),
            lpParameters: PCWSTR(arguments.as_ptr()),
            lpDirectory: PCWSTR(directory.as_ptr()),
            nShow: if vendor_wizard {
                SW_SHOWNORMAL.0
            } else {
                SW_HIDE.0
            },
            ..Default::default()
        };
        if let Err(error) = unsafe { ShellExecuteExW(&mut launch) } {
            return Err(
                if error.code() == windows::core::HRESULT::from_win32(1223) {
                    ComponentReason::UacCancelled
                } else if error.code() == windows::core::HRESULT::from_win32(5) {
                    ComponentReason::UacDenied
                } else {
                    ComponentReason::HelperFailed
                },
            );
        }
        if launch.hProcess.is_invalid() {
            return Err(ComponentReason::HelperFailed);
        }
        let process = launch.hProcess;
        let wait = unsafe { WaitForSingleObject(process, 600_000) };
        if wait == WAIT_TIMEOUT {
            // Never kill an installer or pretend it rolled back. Retain the
            // verified file and operation lock until its actual termination.
            let handle = process.0 as usize;
            std::thread::spawn(move || {
                let process = HANDLE(handle as *mut _);
                unsafe {
                    WaitForSingleObject(process, u32::MAX);
                    let _ = CloseHandle(process);
                }
                drop(helper);
                drop(guard);
            });
            return Ok(None);
        }
        let mut exit = 1;
        let result = if wait == WAIT_OBJECT_0 {
            unsafe { GetExitCodeProcess(process, &mut exit) }
                .map(|_| Some(exit))
                .map_err(|_| ComponentReason::HelperFailed)
        } else {
            Err(ComponentReason::HelperFailed)
        };
        unsafe {
            let _ = CloseHandle(process);
        }
        result
    }

    pub(super) fn recycle_wizard_directory(path: &Path) {
        use windows::Win32::System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
            COINIT_APARTMENTTHREADED,
        };
        use windows::Win32::UI::Shell::*;
        let path = path.to_owned();
        // An STA avoids changing a host worker's COM apartment. Explicit
        // RECYCLEONDELETE requests recycling instead of a permanent fallback.
        let result = std::thread::spawn(move || -> windows::core::Result<()> {
            unsafe {
                CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
            }
            struct Com;
            impl Drop for Com {
                fn drop(&mut self) {
                    unsafe {
                        CoUninitialize();
                    }
                }
            }
            let _com = Com;
            let mut paths: Vec<u16> = path.as_os_str().encode_wide().collect();
            // canonicalize adds the Win32 verbatim prefix, which Shell's item
            // parser rejects. Only unwrap the verified local drive form.
            if paths.starts_with(&[92, 92, 63, 92]) && paths.get(5) == Some(&58) {
                paths.drain(..4);
            }
            paths.push(0);
            unsafe {
                let item: IShellItem = SHCreateItemFromParsingName(PCWSTR(paths.as_ptr()), None)?;
                let operation: IFileOperation =
                    CoCreateInstance(&FileOperation, None, CLSCTX_INPROC_SERVER)?;
                operation.SetOperationFlags(
                    FOFX_RECYCLEONDELETE
                        | FOF_ALLOWUNDO
                        | FOF_NOCONFIRMATION
                        | FOF_NOERRORUI
                        | FOF_SILENT,
                )?;
                operation.DeleteItem(&item, None)?;
                operation.PerformOperations()?;
                if operation.GetAnyOperationsAborted()?.as_bool() {
                    return Err(windows::core::Error::from_hresult(
                        windows::core::HRESULT::from_win32(1223),
                    ));
                }
            }
            Ok(())
        })
        .join();
        let completed = matches!(result, Ok(Ok(())));
        let error_code = match &result {
            Ok(Err(error)) => error.code().0,
            Err(_) => -1,
            _ => 0,
        };
        #[cfg(test)]
        eprintln!("component_support temporary_cleanup completed={completed} code={error_code}");
        crate::gatt_note(format!(
            "component_support action=temporary_cleanup result={} recycle_bin=true code={error_code}",
            if completed {
                "completed"
            } else {
                "failed_retained"
            }
        ));
    }
    impl Drop for ServiceHandle {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseServiceHandle(self.0);
            }
        }
    }

    pub(super) fn probe_vb_cable() -> Evidence {
        let mut evidence = Evidence {
            installed: None,
            loaded: None,
            endpoints: None,
            restart: false,
            failure: None,
        };
        let result = (|| -> windows::core::Result<()> {
            let manager = ServiceHandle(unsafe { OpenSCManagerW(None, None, SC_MANAGER_CONNECT)? });
            let service =
                match unsafe { OpenServiceW(manager.0, w!("VBAudioVACMME"), SERVICE_QUERY_STATUS) }
                {
                    Ok(handle) => ServiceHandle(handle),
                    Err(error) if error.code() == windows::core::HRESULT::from_win32(1060) => {
                        evidence.installed = Some(false);
                        evidence.loaded = Some(false);
                        return Ok(());
                    }
                    Err(error) if error.code() == windows::core::HRESULT::from_win32(1072) => {
                        evidence.installed = Some(true);
                        // Marked-for-delete can also clear when outstanding
                        // handles close; it does not prove a reboot is needed.
                        evidence.loaded = Some(false);
                        evidence.failure = Some(ComponentReason::OperationInProgress);
                        return Ok(());
                    }
                    Err(error) => return Err(error),
                };
            evidence.installed = Some(true);
            let mut status = SERVICE_STATUS_PROCESS::default();
            let bytes = unsafe {
                std::slice::from_raw_parts_mut(
                    (&mut status as *mut SERVICE_STATUS_PROCESS).cast::<u8>(),
                    std::mem::size_of::<SERVICE_STATUS_PROCESS>(),
                )
            };
            let mut needed = 0;
            unsafe {
                QueryServiceStatusEx(service.0, SC_STATUS_PROCESS_INFO, Some(bytes), &mut needed)?;
            }
            evidence.loaded = Some(status.dwCurrentState == SERVICE_RUNNING);
            Ok(())
        })();
        if let Err(error) = result {
            evidence.failure = Some(if error.code() == windows::core::HRESULT::from_win32(5) {
                ComponentReason::AccessDenied
            } else {
                ComponentReason::DetectionFailed
            });
            return evidence;
        }
        // Only active endpoints are enumerated; no audio stream is opened.
        let endpoints = (|| -> Result<bool, ()> {
            wasapi::initialize_mta().ok().map_err(|_| ())?;
            struct Com;
            impl Drop for Com {
                fn drop(&mut self) {
                    wasapi::deinitialize();
                }
            }
            let _com = Com;
            let enumerator = wasapi::DeviceEnumerator::new().map_err(|_| ())?;
            let mut found = [false; 2];
            for (index, direction) in [wasapi::Direction::Render, wasapi::Direction::Capture]
                .iter()
                .enumerate()
            {
                let collection = enumerator
                    .get_device_collection(direction)
                    .map_err(|_| ())?;
                for device in &collection {
                    let name = device.map_err(|_| ())?.get_friendlyname().map_err(|_| ())?;
                    let expected = if index == 0 {
                        "cable input"
                    } else {
                        "cable output"
                    };
                    let name = name.to_ascii_lowercase();
                    found[index] |= name.starts_with(expected) && name.contains("vb-audio");
                }
            }
            Ok(found.into_iter().all(|value| value))
        })();
        match endpoints {
            Ok(ready) => evidence.endpoints = Some(ready),
            Err(()) => evidence.failure = Some(ComponentReason::DetectionFailed),
        }
        evidence
    }

    pub(super) fn verify_signature_and_version(
        path: &Path,
        file: &File,
        artifact: &TrustedArtifact,
    ) -> Result<(), ComponentReason> {
        use windows::Win32::Security::{Cryptography::*, WinTrust::*};
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut info = WINTRUST_FILE_INFO {
            cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
            pcwszFilePath: PCWSTR(wide.as_ptr()),
            hFile: HANDLE(file.as_raw_handle()),
            ..Default::default()
        };
        let mut data = WINTRUST_DATA {
            cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
            dwUIChoice: WTD_UI_NONE,
            fdwRevocationChecks: WTD_REVOKE_WHOLECHAIN,
            dwUnionChoice: WTD_CHOICE_FILE,
            dwStateAction: WTD_STATEACTION_VERIFY,
            Anonymous: WINTRUST_DATA_0 { pFile: &mut info },
            ..Default::default()
        };
        let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
        let code = unsafe {
            WinVerifyTrust(
                windows::Win32::Foundation::HWND::default(),
                &mut action,
                (&mut data as *mut WINTRUST_DATA).cast(),
            )
        };
        let result = (|| {
            if code != 0 {
                return Err(ComponentReason::SignatureInvalid);
            }
            let provider = unsafe { WTHelperProvDataFromStateData(data.hWVTStateData) };
            if provider.is_null() {
                return Err(ComponentReason::SignatureInvalid);
            }
            let signer = unsafe { WTHelperGetProvSignerFromChain(provider, 0, false, 0) };
            if signer.is_null() {
                return Err(ComponentReason::SignatureInvalid);
            }
            let certificate = unsafe { WTHelperGetProvCertFromChain(signer, 0) };
            if certificate.is_null() {
                return Err(ComponentReason::SignatureInvalid);
            }
            let cert = unsafe { (*certificate).pCert };
            if cert.is_null() {
                return Err(ComponentReason::SignatureInvalid);
            }
            let mut thumbprint = [0u8; 32];
            let mut length = 32;
            unsafe {
                CertGetCertificateContextProperty(
                    cert,
                    CERT_SHA256_HASH_PROP_ID,
                    Some(thumbprint.as_mut_ptr().cast()),
                    &mut length,
                )
            }
            .map_err(|_| ComponentReason::SignatureInvalid)?;
            if length != 32 || thumbprint != artifact.signer_sha256 {
                return Err(ComponentReason::PublisherMismatch);
            }
            let length =
                unsafe { CertGetNameStringW(cert, CERT_NAME_SIMPLE_DISPLAY_TYPE, 0, None, None) };
            if length == 0 || length > 4096 {
                return Err(ComponentReason::PublisherMismatch);
            }
            let mut publisher = vec![0u16; length as usize];
            unsafe {
                CertGetNameStringW(
                    cert,
                    CERT_NAME_SIMPLE_DISPLAY_TYPE,
                    0,
                    None,
                    Some(&mut publisher),
                );
            }
            if String::from_utf16_lossy(&publisher[..publisher.len() - 1]) != artifact.publisher {
                return Err(ComponentReason::PublisherMismatch);
            }
            verify_version(PCWSTR(wide.as_ptr()), artifact.file_version)
        })();
        data.dwStateAction = WTD_STATEACTION_CLOSE;
        unsafe {
            WinVerifyTrust(
                windows::Win32::Foundation::HWND::default(),
                &mut action,
                (&mut data as *mut WINTRUST_DATA).cast(),
            );
        }
        result
    }

    fn verify_version(path: PCWSTR, expected: [u16; 4]) -> Result<(), ComponentReason> {
        use windows::Win32::Storage::FileSystem::*;
        let length = unsafe { GetFileVersionInfoSizeW(path, None) };
        if length == 0 || length > 1024 * 1024 {
            return Err(ComponentReason::VersionMismatch);
        }
        let mut bytes = vec![0u8; length as usize];
        unsafe { GetFileVersionInfoW(path, None, length, bytes.as_mut_ptr().cast()) }
            .map_err(|_| ComponentReason::VersionMismatch)?;
        let mut pointer = std::ptr::null_mut();
        let mut length = 0;
        if !unsafe { VerQueryValueW(bytes.as_ptr().cast(), w!("\\"), &mut pointer, &mut length) }
            .as_bool()
            || pointer.is_null()
            || length < std::mem::size_of::<VS_FIXEDFILEINFO>() as u32
        {
            return Err(ComponentReason::VersionMismatch);
        }
        let version = unsafe { std::ptr::read_unaligned(pointer.cast::<VS_FIXEDFILEINFO>()) };
        let actual = [
            (version.dwFileVersionMS >> 16) as u16,
            version.dwFileVersionMS as u16,
            (version.dwFileVersionLS >> 16) as u16,
            version.dwFileVersionLS as u16,
        ];
        if actual != expected {
            return Err(ComponentReason::VersionMismatch);
        }
        Ok(())
    }
}

#[cfg(windows)]
mod probe {
    use serde::Serialize;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::io::Write;
    use std::mem::{offset_of, size_of};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};
    use windows::core::{w, BOOL, PCWSTR, PWSTR};
    use windows::Win32::Devices::DeviceAndDriverInstallation::{
        CM_Get_DevNode_PropertyW, CM_Get_Device_Interface_PropertyW, CM_Locate_DevNodeW,
        CM_LOCATE_DEVNODE_NORMAL, CR_BUFFER_SMALL, CR_SUCCESS,
    };
    use windows::Win32::Devices::Properties::{
        DEVPKEY_Device_HardwareIds, DEVPKEY_Device_InstanceId, DEVPROPTYPE, DEVPROP_TYPE_STRING,
        DEVPROP_TYPE_STRING_LIST,
    };
    use windows::Win32::Foundation::{
        CloseHandle, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM,
    };
    use windows::Win32::System::Console::SetConsoleCtrlHandler;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Threading::{OpenEventW, SetEvent, EVENT_MODIFY_STATE};
    use windows::Win32::UI::Input::{
        GetRawInputData, GetRawInputDeviceInfoW, GetRawInputDeviceList,
        GetRegisteredRawInputDevices, RegisterRawInputDevices, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE,
        RAWINPUTDEVICELIST, RAWINPUTHEADER, RAWKEYBOARD, RIDEV_DEVNOTIFY, RIDEV_INPUTSINK,
        RIDEV_REMOVE, RIDI_DEVICENAME, RID_HEADER, RID_INPUT, RIM_TYPEKEYBOARD,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, KillTimer,
        PostMessageW, PostQuitMessage, RegisterClassW, SetTimer, UnregisterClassW, HWND_MESSAGE,
        MSG, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE, WM_DESTROY, WM_INPUT, WM_INPUT_DEVICE_CHANGE,
        WM_TIMER, WNDCLASSW,
    };

    const MODEL: &str =
        "HID\\{00001812-0000-1000-8000-00805f9b34fb}_Dev_VID&012717_PID&32b8_REV&00a4";
    const CLASS: PCWSTR = w!("WirelessMicReadonlyTvRawInputProbe");
    static CANCEL: AtomicBool = AtomicBool::new(false);

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "snake_case")]
    enum Origin {
        #[serde(rename = "model_candidate")]
        Target,
        Other,
        Unknown,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "snake_case")]
    enum Step {
        Tv,
        Backtick,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
    struct Failure {
        stage: &'static str,
        code: u32,
    }

    type Result<T> = std::result::Result<T, Failure>;

    fn failure(stage: &'static str) -> Failure {
        Failure { stage, code: 0 }
    }
    fn win_failure(stage: &'static str, error: windows::core::Error) -> Failure {
        Failure {
            stage,
            code: error.code().0 as u32,
        }
    }

    #[derive(Debug, Serialize)]
    struct Candidate {
        source: Origin,
        device_ordinal: u32,
        make_code: u16,
        extended_flags: u16,
        make: u32,
        break_count: u32,
        repeated_make: u32,
    }

    #[derive(Debug, Serialize)]
    struct DeviceChange {
        kind: &'static str,
        snapshot_origin: Origin,
        snapshot_ordinal: Option<u32>,
        processed_elapsed_ms: u128,
        processing_phase: &'static str,
    }

    #[derive(Debug, Serialize)]
    struct Sample {
        step: Step,
        candidate: Option<Candidate>,
        target_events: u32,
        other_events: u32,
        unknown_events: u32,
        ignored_events: u32,
        orphan_break: u32,
        ambiguous: bool,
        cancelled: bool,
        device_changed: bool,
        first_device_change: Option<DeviceChange>,
    }

    impl Sample {
        fn new(step: Step) -> Self {
            Self {
                step,
                candidate: None,
                target_events: 0,
                other_events: 0,
                unknown_events: 0,
                ignored_events: 0,
                orphan_break: 0,
                ambiguous: false,
                cancelled: false,
                device_changed: false,
                first_device_change: None,
            }
        }

        fn observe(&mut self, origin: Origin, device_ordinal: u32, keyboard: Option<RAWKEYBOARD>) {
            match origin {
                Origin::Target => self.target_events += 1,
                Origin::Other => self.other_events += 1,
                Origin::Unknown => self.unknown_events += 1,
            }
            let accepted_source = matches!(
                (self.step, origin),
                (Step::Tv, Origin::Target) | (Step::Backtick, Origin::Other)
            );
            if !accepted_source {
                return;
            }
            let keyboard = keyboard.expect("accepted source has decoded packet");
            if self.step == Step::Backtick && keyboard.VKey != 0xc0 {
                self.ignored_events += 1;
                return;
            }
            let flags = keyboard.Flags;
            if keyboard.MakeCode == 0
                || keyboard.MakeCode == 0xff
                || flags & !7 != 0
                || flags & 6 == 6
            {
                self.ambiguous = true;
                return;
            }
            let released = flags & 1 != 0;
            let extended = flags & 6;
            if let Some(candidate) = &mut self.candidate {
                if candidate.make_code != keyboard.MakeCode
                    || candidate.extended_flags != extended
                    || candidate.source != origin
                    || candidate.device_ordinal != device_ordinal
                {
                    self.ambiguous = true;
                } else if released {
                    candidate.break_count += 1;
                    if candidate.break_count > 1 {
                        self.ambiguous = true;
                    }
                } else if candidate.break_count == 0 {
                    candidate.repeated_make += 1;
                } else {
                    self.ambiguous = true;
                }
            } else if released {
                self.orphan_break += 1;
            } else {
                self.candidate = Some(Candidate {
                    source: origin,
                    device_ordinal,
                    make_code: keyboard.MakeCode,
                    extended_flags: extended,
                    make: 1,
                    break_count: 0,
                    repeated_make: 0,
                });
            }
        }

        fn outcome(&self) -> &'static str {
            if self.cancelled {
                "cancelled"
            } else if self.device_changed {
                "device_changed"
            } else if self.ambiguous || self.orphan_break > 0 {
                "ambiguous"
            } else if self
                .candidate
                .as_ref()
                .is_some_and(|value| value.break_count == 1)
            {
                "candidate_observed"
            } else {
                "insufficient"
            }
        }
    }

    fn source_for(handle: usize, sources: &HashMap<usize, (Origin, u32)>) -> (Origin, u32) {
        sources
            .get(&handle)
            .copied()
            .unwrap_or((Origin::Unknown, 0))
    }

    fn decode(bytes: &[u8], origin: Origin, step: Step) -> Result<Option<RAWKEYBOARD>> {
        if bytes.len() < size_of::<RAWINPUTHEADER>() {
            return Err(failure("raw_header_truncated"));
        }
        let header = unsafe { bytes.as_ptr().cast::<RAWINPUTHEADER>().read_unaligned() };
        if header.dwSize as usize != bytes.len() {
            return Err(failure("raw_size_mismatch"));
        }
        if header.dwType != RIM_TYPEKEYBOARD.0 {
            return Err(failure("raw_type_mismatch"));
        }
        if bytes.len() < size_of::<RAWINPUTHEADER>() + size_of::<RAWKEYBOARD>() {
            return Err(failure("raw_keyboard_truncated"));
        }
        if !matches!(
            (step, origin),
            (Step::Tv, Origin::Target) | (Step::Backtick, Origin::Other)
        ) {
            return Ok(None);
        }
        Ok(Some(unsafe {
            bytes
                .as_ptr()
                .add(size_of::<RAWINPUTHEADER>())
                .cast::<RAWKEYBOARD>()
                .read_unaligned()
        }))
    }

    fn utf16(bytes: &[u8]) -> Result<Vec<u16>> {
        if bytes.len() % 2 != 0 {
            return Err(failure("property_length"));
        }
        let words: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|value| u16::from_le_bytes([value[0], value[1]]))
            .collect();
        if words.last() != Some(&0) {
            return Err(failure("property_terminator"));
        }
        Ok(words)
    }

    unsafe fn origin(handle: HANDLE) -> Result<Origin> {
        let mut length = 0;
        if GetRawInputDeviceInfoW(Some(handle), RIDI_DEVICENAME, None, &mut length) == u32::MAX {
            return Err(win_failure(
                "raw_name_size",
                windows::core::Error::from_thread(),
            ));
        }
        if length == 0 || length > 32768 {
            return Err(failure("raw_name_length"));
        }
        let mut path = vec![0u16; length as usize];
        if GetRawInputDeviceInfoW(
            Some(handle),
            RIDI_DEVICENAME,
            Some(path.as_mut_ptr().cast()),
            &mut length,
        ) == u32::MAX
        {
            return Err(win_failure("raw_name", windows::core::Error::from_thread()));
        }
        if path.last() != Some(&0) {
            return Err(failure("raw_name_terminator"));
        }
        let mut property_type = DEVPROPTYPE::default();
        let mut size = 0;
        let status = CM_Get_Device_Interface_PropertyW(
            PCWSTR(path.as_ptr()),
            &DEVPKEY_Device_InstanceId,
            &mut property_type,
            None,
            &mut size,
            0,
        );
        if status != CR_BUFFER_SMALL || size == 0 || size > 65536 {
            return Err(Failure {
                stage: "interface_instance_size",
                code: status.0,
            });
        }
        let mut instance_bytes = vec![0u8; size as usize];
        let status = CM_Get_Device_Interface_PropertyW(
            PCWSTR(path.as_ptr()),
            &DEVPKEY_Device_InstanceId,
            &mut property_type,
            Some(instance_bytes.as_mut_ptr()),
            &mut size,
            0,
        );
        if status != CR_SUCCESS || property_type != DEVPROP_TYPE_STRING {
            return Err(Failure {
                stage: "interface_instance",
                code: status.0,
            });
        }
        instance_bytes.truncate(size as usize);
        let mut instance = utf16(&instance_bytes)?;
        let mut devnode = 0;
        let status = CM_Locate_DevNodeW(
            &mut devnode,
            PWSTR(instance.as_mut_ptr()),
            CM_LOCATE_DEVNODE_NORMAL,
        );
        if status != CR_SUCCESS {
            return Err(Failure {
                stage: "locate_devnode",
                code: status.0,
            });
        }
        size = 0;
        let status = CM_Get_DevNode_PropertyW(
            devnode,
            &DEVPKEY_Device_HardwareIds,
            &mut property_type,
            None,
            &mut size,
            0,
        );
        if status != CR_BUFFER_SMALL || size == 0 || size > 65536 {
            return Err(Failure {
                stage: "hardware_ids_size",
                code: status.0,
            });
        }
        let mut ids = vec![0u8; size as usize];
        let status = CM_Get_DevNode_PropertyW(
            devnode,
            &DEVPKEY_Device_HardwareIds,
            &mut property_type,
            Some(ids.as_mut_ptr()),
            &mut size,
            0,
        );
        if status != CR_SUCCESS || property_type != DEVPROP_TYPE_STRING_LIST {
            return Err(Failure {
                stage: "hardware_ids",
                code: status.0,
            });
        }
        ids.truncate(size as usize);
        let words = utf16(&ids)?;
        let mut matched = false;
        for id in words.split(|value| *value == 0) {
            let id = String::from_utf16(id).map_err(|_| failure("hardware_ids_encoding"))?;
            matched |= id.eq_ignore_ascii_case(MODEL);
        }
        Ok(if matched {
            Origin::Target
        } else {
            Origin::Other
        })
    }

    unsafe fn enumerate() -> Result<(HashMap<usize, (Origin, u32)>, serde_json::Value)> {
        let mut count = 0;
        let item_size = size_of::<RAWINPUTDEVICELIST>() as u32;
        if GetRawInputDeviceList(None, &mut count, item_size) == u32::MAX {
            return Err(win_failure(
                "device_list_size",
                windows::core::Error::from_thread(),
            ));
        }
        let mut devices = vec![RAWINPUTDEVICELIST::default(); count as usize];
        if count == 0 {
            return Err(failure("no_raw_devices"));
        }
        let written = GetRawInputDeviceList(Some(devices.as_mut_ptr()), &mut count, item_size);
        if written == u32::MAX {
            return Err(win_failure(
                "device_list",
                windows::core::Error::from_thread(),
            ));
        }
        if written as usize > devices.len() {
            return Err(failure("device_list_length"));
        }
        let mut sources = HashMap::new();
        let mut unknown_errors = Vec::new();
        for device in devices
            .into_iter()
            .take(written as usize)
            .filter(|value| value.dwType == RIM_TYPEKEYBOARD)
        {
            let verdict = match origin(device.hDevice) {
                Ok(value) => value,
                Err(error) => {
                    unknown_errors.push(error);
                    Origin::Unknown
                }
            };
            let ordinal = sources.len() as u32 + 1;
            sources.insert(device.hDevice.0 as usize, (verdict, ordinal));
        }
        let target_count = sources
            .values()
            .filter(|value| value.0 == Origin::Target)
            .count();
        let inventory = serde_json::json!({ "event": "inventory", "keyboard_devices": sources.len(), "model_candidates": target_count, "unknown_devices": unknown_errors.len(), "unknown_errors": unknown_errors,
                "source_association": "raw_hdevice_interface_to_cm_devnode", "selection_basis": "unique_model_hardware_id",
                "selected_remote_identity_proven": false, "filter_binding_proven": false });
        if target_count != 1 {
            return Err(failure("target_not_unique"));
        }
        Ok((sources, inventory))
    }

    fn registered_sources<Source>(
        register: impl FnOnce() -> Result<()>,
        enumerate: impl FnOnce() -> Result<Source>,
        ready: impl FnOnce() -> Result<()>,
    ) -> Result<Source> {
        register()?;
        let sources = enumerate()?;
        ready()?;
        Ok(sources)
    }

    fn signal_ready(name: &str) -> Result<()> {
        let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        unsafe {
            let handle = OpenEventW(EVENT_MODIFY_STATE, false, PCWSTR(name.as_ptr()))
                .map_err(|error| win_failure("ready_open", error))?;
            let signaled = SetEvent(handle).map_err(|error| win_failure("ready_set", error));
            let closed = CloseHandle(handle).map_err(|error| win_failure("ready_close", error));
            signaled?;
            closed
        }
    }

    struct Context {
        sources: HashMap<usize, (Origin, u32)>,
        sample: Sample,
        started: Instant,
        limit: Duration,
        error: Option<Failure>,
    }
    thread_local! { static CONTEXT: RefCell<Option<Context>> = const { RefCell::new(None) }; }

    fn limits_reached(context: &mut Context, now: Instant, cancelled: bool) -> bool {
        context.sample.cancelled |= cancelled;
        context.sample.cancelled
            || context.sample.device_changed
            || now.duration_since(context.started) >= context.limit
    }

    fn note_device_change(context: &mut Context, kind: usize, handle: usize, now: Instant) {
        context.sample.device_changed = true;
        if context.sample.first_device_change.is_none() {
            let snapshot = context.sources.get(&handle);
            context.sample.first_device_change = Some(DeviceChange {
                kind: match kind {
                    1 => "arrival",
                    2 => "removal",
                    _ => "unknown",
                },
                snapshot_origin: snapshot.map_or(Origin::Unknown, |source| source.0),
                snapshot_ordinal: snapshot.map(|source| source.1),
                processed_elapsed_ms: now.duration_since(context.started).as_millis(),
                processing_phase: "window_proc",
            });
        }
    }

    fn start_capture(
        starting: impl FnOnce() -> Result<()>,
        capture: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        starting()?;
        capture()
    }

    unsafe fn receive(handle: HRAWINPUT, context: &mut Context) -> Result<()> {
        if limits_reached(context, Instant::now(), CANCEL.load(Ordering::SeqCst)) {
            return Ok(());
        }
        let header_size = size_of::<RAWINPUTHEADER>() as u32;
        let mut header = RAWINPUTHEADER::default();
        let mut length = header_size;
        let copied = GetRawInputData(
            handle,
            RID_HEADER,
            Some((&mut header as *mut RAWINPUTHEADER).cast()),
            &mut length,
            header_size,
        );
        if copied == u32::MAX {
            return Err(win_failure(
                "raw_header",
                windows::core::Error::from_thread(),
            ));
        }
        if copied != header_size
            || header.dwSize as usize != size_of::<RAWINPUTHEADER>() + size_of::<RAWKEYBOARD>()
            || header.dwType != RIM_TYPEKEYBOARD.0
        {
            return Err(failure("raw_header_contract"));
        }
        let (verdict, ordinal) = source_for(header.hDevice.0 as usize, &context.sources);
        if !matches!(
            (context.sample.step, verdict),
            (Step::Tv, Origin::Target) | (Step::Backtick, Origin::Other)
        ) {
            context.sample.observe(verdict, ordinal, None);
            return Ok(());
        }
        let mut aligned = vec![0usize; (header.dwSize as usize).div_ceil(size_of::<usize>())];
        length = header.dwSize;
        if limits_reached(context, Instant::now(), CANCEL.load(Ordering::SeqCst)) {
            return Ok(());
        }
        let copied = GetRawInputData(
            handle,
            RID_INPUT,
            Some(aligned.as_mut_ptr().cast()),
            &mut length,
            header_size,
        );
        if copied == u32::MAX {
            return Err(win_failure(
                "raw_input",
                windows::core::Error::from_thread(),
            ));
        }
        if copied != header.dwSize || length != header.dwSize {
            return Err(failure("raw_copied_length"));
        }
        let bytes = std::slice::from_raw_parts(aligned.as_ptr().cast::<u8>(), copied as usize);
        let packet_header = bytes.as_ptr().cast::<RAWINPUTHEADER>().read_unaligned();
        if packet_header.hDevice != header.hDevice {
            return Err(failure("raw_source_changed"));
        }
        context.sample.observe(
            verdict,
            ordinal,
            decode(bytes, verdict, context.sample.step)?,
        );
        Ok(())
    }

    unsafe extern "system" fn window_proc(
        window: HWND,
        message: u32,
        parameter: WPARAM,
        data: LPARAM,
    ) -> LRESULT {
        match message {
            WM_INPUT => {
                let stop = CONTEXT.with(|slot| {
                    let mut slot = slot.borrow_mut();
                    let context = slot
                        .as_mut()
                        .expect("context installed before registration");
                    if let Err(error) = receive(HRAWINPUT(data.0 as *mut _), context) {
                        context.error = Some(error);
                    }
                    context.error.is_some()
                        || context
                            .sample
                            .candidate
                            .as_ref()
                            .is_some_and(|value| value.break_count == 1)
                        || limits_reached(context, Instant::now(), CANCEL.load(Ordering::SeqCst))
                });
                if stop {
                    let _ = PostMessageW(Some(window), WM_CLOSE, WPARAM(0), LPARAM(0));
                }
                DefWindowProcW(window, message, parameter, data)
            }
            WM_INPUT_DEVICE_CHANGE => {
                CONTEXT.with(|slot| {
                    note_device_change(
                        slot.borrow_mut().as_mut().expect("context installed"),
                        parameter.0,
                        data.0 as usize,
                        Instant::now(),
                    )
                });
                let _ = PostMessageW(Some(window), WM_CLOSE, WPARAM(0), LPARAM(0));
                LRESULT(0)
            }
            WM_TIMER => LRESULT(0),
            WM_CLOSE => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            WM_DESTROY => LRESULT(0),
            _ => DefWindowProcW(window, message, parameter, data),
        }
    }

    unsafe extern "system" fn ctrl_handler(kind: u32) -> BOOL {
        if kind <= 1 {
            CANCEL.store(true, Ordering::SeqCst);
            BOOL(1)
        } else {
            BOOL(0)
        }
    }

    fn emit(value: &impl Serialize) -> Result<()> {
        let mut output = std::io::stdout().lock();
        emit_to(&mut output, value)
    }

    fn emit_to(output: &mut impl Write, value: &impl Serialize) -> Result<()> {
        serde_json::to_writer(&mut *output, value).map_err(|_| failure("output"))?;
        writeln!(output).map_err(|_| failure("output"))?;
        output.flush().map_err(|_| failure("output"))
    }

    fn bounds_fixture(started: Instant) -> Context {
        Context {
            sources: HashMap::new(),
            sample: Sample::new(Step::Tv),
            started,
            limit: Duration::from_millis(5),
            error: None,
        }
    }

    fn verify_busy_deadline() {
        let started = Instant::now();
        for message in [WM_INPUT, 0x8001] {
            let mut context = bounds_fixture(started);
            let mut dispatched = Vec::new();
            for offset in 0..20 {
                if limits_reached(&mut context, started + Duration::from_millis(offset), false) {
                    break;
                }
                dispatched.push(message);
            }
            assert_eq!(
                dispatched,
                vec![message; 5],
                "busy messages starved the capture deadline"
            );
        }
    }

    fn verify_busy_cancel() {
        let started = Instant::now();
        for message in [WM_INPUT, 0x8001] {
            let mut context = bounds_fixture(started);
            let mut dispatched = Vec::new();
            for offset in 0..4 {
                if limits_reached(&mut context, started, offset == 2) {
                    break;
                }
                dispatched.push(message);
            }
            assert_eq!(
                dispatched,
                vec![message; 2],
                "busy messages starved cancellation"
            );
            assert!(context.sample.cancelled);
        }
    }

    fn verify_pre_read_bounds() {
        let started = Instant::now();
        for cancelled in [false, true] {
            let mut context = bounds_fixture(started);
            assert!(!limits_reached(&mut context, started, false));
            let now = if cancelled {
                started
            } else {
                started + context.limit
            };
            let mut reads = 0;
            if !limits_reached(&mut context, now, cancelled) {
                reads += 1;
            }
            assert_eq!(reads, 0, "raw read began after the capture bound");
        }
    }

    fn verify_blocked_starting() {
        use std::sync::{mpsc, Arc};
        struct BlockedOutput {
            entered: mpsc::Sender<()>,
            release: mpsc::Receiver<()>,
        }
        impl Write for BlockedOutput {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                self.entered.send(()).unwrap();
                self.release.recv().unwrap();
                Ok(())
            }
        }
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let registration_calls = calls.clone();
        let (entered, blocked) = mpsc::channel();
        let (release, released) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let mut output = BlockedOutput {
                entered,
                release: released,
            };
            start_capture(
                || emit_to(&mut output, &serde_json::json!({ "event": "starting" })),
                || {
                    registration_calls.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                },
            )
        });
        blocked.recv_timeout(Duration::from_secs(2)).unwrap();
        let while_blocked = calls.load(Ordering::SeqCst);
        release.send(()).unwrap();
        worker.join().unwrap().unwrap();
        assert_eq!(
            while_blocked, 0,
            "registration preceded starting output completion"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    fn verify_starting_failure() {
        let mut registration_calls = 0;
        let result = start_capture(
            || Err(failure("output")),
            || {
                registration_calls += 1;
                Ok(())
            },
        );
        assert_eq!(result, Err(failure("output")));
        assert_eq!(registration_calls, 0);
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn notification_registration_precedes_source_snapshot() {
            let calls = RefCell::new(Vec::new());
            let result = registered_sources(
                || {
                    calls.borrow_mut().push("registered");
                    Ok(())
                },
                || {
                    calls.borrow_mut().push("enumerated");
                    Ok(7)
                },
                || Ok(()),
            );
            assert_eq!(result, Ok(7));
            assert_eq!(*calls.borrow(), ["registered", "enumerated"]);
        }

        #[test]
        fn enumeration_failure_keeps_registration_cleanup_required() {
            let registered = std::cell::Cell::new(false);
            let result: Result<()> = registered_sources(
                || {
                    registered.set(true);
                    Ok(())
                },
                || Err(failure("synthetic_enumeration")),
                || Ok(()),
            );
            assert_eq!(result, Err(failure("synthetic_enumeration")));
            assert!(registered.get());
        }

        #[test]
        fn failed_registration_never_builds_source_snapshot() {
            let enumerated = std::cell::Cell::new(false);
            let result = registered_sources(
                || Err(failure("synthetic_registration")),
                || {
                    enumerated.set(true);
                    Ok(())
                },
                || Ok(()),
            );
            assert_eq!(result, Err(failure("synthetic_registration")));
            assert!(!enumerated.get());
        }

        #[test]
        fn model_candidate_is_not_verified_remote() {
            let serialized = serde_json::to_string(&Origin::Target).unwrap();
            assert_eq!(serialized, "\"model_candidate\"");
        }

        #[test]
        fn readiness_follows_registration_and_installed_map() {
            let calls = RefCell::new(Vec::new());
            let result = registered_sources(
                || {
                    calls.borrow_mut().push("registered");
                    Ok(())
                },
                || {
                    calls.borrow_mut().push("map_installed");
                    Ok(())
                },
                || {
                    calls.borrow_mut().push("ready");
                    Ok(())
                },
            );
            assert_eq!(result, Ok(()));
            assert_eq!(*calls.borrow(), ["registered", "map_installed", "ready"]);
        }

        #[test]
        fn readiness_failure_preserves_cleanup_requirement() {
            let registered = std::cell::Cell::new(false);
            let result = registered_sources(
                || {
                    registered.set(true);
                    Ok(())
                },
                || Ok(()),
                || Err(failure("synthetic_ready")),
            );
            assert_eq!(result, Err(failure("synthetic_ready")));
            assert!(registered.get());
        }

        #[test]
        fn readiness_named_event_child() {
            if let Ok(name) = std::env::var("G2_PROBE_READY_TEST_EVENT") {
                signal_ready(&name).expect("named ready event");
            }
        }

        #[test]
        fn readiness_named_event_cross_process() {
            use windows::Win32::Foundation::WAIT_OBJECT_0;
            use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};
            let name = format!("Local\\G2ProbeReadyTest-{}", std::process::id());
            let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
            let handle = unsafe { CreateEventW(None, true, false, PCWSTR(wide.as_ptr())) }
                .expect("create named event");
            let child = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "probe::tests::readiness_named_event_child"])
                .env("G2_PROBE_READY_TEST_EVENT", &name)
                .spawn();
            let observed = unsafe { WaitForSingleObject(handle, 5000) };
            unsafe { CloseHandle(handle) }.expect("close named event");
            let status = child.expect("child process").wait().expect("child exit");
            assert!(status.success());
            assert_eq!(observed, WAIT_OBJECT_0);
        }

        #[test]
        fn device_change_invalidates_old_source_before_next_read() {
            let started = Instant::now();
            let mut context = bounds_fixture(started);
            context.sample.device_changed = true;
            assert!(limits_reached(&mut context, started, false));
            assert_eq!(context.sample.outcome(), "device_changed");
        }

        #[test]
        fn device_change_projects_kind_source_and_processing_time() {
            let started = Instant::now();
            for (kind, origin, expected_kind, expected_origin) in [
                (1, Origin::Target, "arrival", "model_candidate"),
                (2, Origin::Other, "removal", "other"),
                (99, Origin::Unknown, "unknown", "unknown"),
            ] {
                let mut context = bounds_fixture(started);
                context.sources.insert(54321, (origin, 3));
                note_device_change(
                    &mut context,
                    kind,
                    54321,
                    started + Duration::from_millis(7),
                );
                let json = serde_json::to_value(&context.sample).unwrap();
                assert_eq!(json["first_device_change"]["kind"], expected_kind);
                assert_eq!(
                    json["first_device_change"]["snapshot_origin"],
                    expected_origin
                );
                assert_eq!(json["first_device_change"]["snapshot_ordinal"], 3);
                assert_eq!(json["first_device_change"]["processed_elapsed_ms"], 7);
                assert_eq!(
                    json["first_device_change"]["processing_phase"],
                    "window_proc"
                );
                assert!(!json.to_string().contains("54321"));
                assert!(limits_reached(
                    &mut context,
                    started + Duration::from_millis(7),
                    false
                ));
            }
        }

        #[test]
        fn device_change_preserves_first_unknown_notification_and_stops() {
            let started = Instant::now();
            let mut context = bounds_fixture(started);
            note_device_change(&mut context, 2, 987654, started);
            note_device_change(&mut context, 1, 123456, started + Duration::from_millis(8));
            let json = serde_json::to_value(&context.sample).unwrap();
            assert_eq!(json["first_device_change"]["kind"], "removal");
            assert_eq!(json["first_device_change"]["snapshot_origin"], "unknown");
            assert_eq!(
                json["first_device_change"]["snapshot_ordinal"],
                serde_json::Value::Null
            );
            assert_eq!(json["first_device_change"]["processed_elapsed_ms"], 0);
            assert!(!json.to_string().contains("987654"));
            assert!(!json.to_string().contains("123456"));
            assert_eq!(context.sample.outcome(), "device_changed");
            assert!(limits_reached(&mut context, started, false));
        }

        #[test]
        fn missing_or_other_binding_never_decodes_remote_candidate() {
            let sources = HashMap::from([(1, (Origin::Target, 1)), (2, (Origin::Other, 2))]);
            assert_eq!(source_for(3, &sources), (Origin::Unknown, 0));
            assert_eq!(source_for(2, &sources), (Origin::Other, 2));
            let bytes = fixture(3, 0x29, 0, 0xc0);
            assert!(decode(&bytes, Origin::Unknown, Step::Tv).unwrap().is_none());
            assert!(decode(&bytes, Origin::Other, Step::Tv).unwrap().is_none());
        }

        #[test]
        fn busy_messages_respect_deadline() {
            verify_busy_deadline();
        }
        #[test]
        fn busy_messages_respect_cancel() {
            verify_busy_cancel();
        }
        #[test]
        fn raw_read_respects_deadline_and_cancel() {
            verify_pre_read_bounds();
        }
        #[test]
        fn blocked_starting_has_no_registration() {
            verify_blocked_starting();
        }
        #[test]
        fn failed_starting_has_no_registration() {
            verify_starting_failure();
        }
    }

    unsafe fn registration_present() -> Result<bool> {
        let mut count = 0;
        let size = size_of::<RAWINPUTDEVICE>() as u32;
        if GetRegisteredRawInputDevices(None, &mut count, size) == u32::MAX {
            return Err(win_failure(
                "registration_size",
                windows::core::Error::from_thread(),
            ));
        }
        if count == 0 {
            return Ok(false);
        }
        let mut devices = vec![RAWINPUTDEVICE::default(); count as usize];
        let written = GetRegisteredRawInputDevices(Some(devices.as_mut_ptr()), &mut count, size);
        if written == u32::MAX {
            return Err(win_failure(
                "registration_readback",
                windows::core::Error::from_thread(),
            ));
        }
        if written as usize > devices.len() {
            return Err(failure("registration_length"));
        }
        Ok(devices
            .iter()
            .take(written as usize)
            .any(|value| value.usUsagePage == 1 && value.usUsage == 6))
    }

    unsafe fn capture(step: Step, seconds: u64, ready_event: &str) -> Result<()> {
        if registration_present()? {
            return Err(failure("existing_process_registration"));
        }
        let module = GetModuleHandleW(None).map_err(|error| win_failure("module", error))?;
        let instance = HINSTANCE(module.0);
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: CLASS,
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            return Err(win_failure(
                "window_class",
                windows::core::Error::from_thread(),
            ));
        }
        let window_result = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            CLASS,
            CLASS,
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(instance),
            None,
        );
        let window = match window_result {
            Ok(value) => value,
            Err(error) => {
                UnregisterClassW(CLASS, Some(instance))
                    .map_err(|error| win_failure("unregister_class", error))?;
                return Err(win_failure("create_window", error));
            }
        };
        CONTEXT.with(|slot| {
            *slot.borrow_mut() = Some(Context {
                sources: HashMap::new(),
                sample: Sample::new(step),
                started: Instant::now(),
                limit: Duration::from_secs(seconds),
                error: None,
            })
        });
        let mut registered = false;
        let mut handler = false;
        let mut timer = false;
        let mut inventory = None;
        let session = (|| -> Result<()> {
            SetConsoleCtrlHandler(Some(ctrl_handler), true)
                .map_err(|error| win_failure("ctrl_handler", error))?;
            handler = true;
            if SetTimer(Some(window), 1, 50, None) == 0 {
                return Err(win_failure("timer", windows::core::Error::from_thread()));
            }
            timer = true;
            let summary = registered_sources(
                || {
                    RegisterRawInputDevices(
                        &[RAWINPUTDEVICE {
                            usUsagePage: 1,
                            usUsage: 6,
                            dwFlags: RIDEV_INPUTSINK | RIDEV_DEVNOTIFY,
                            hwndTarget: window,
                        }],
                        size_of::<RAWINPUTDEVICE>() as u32,
                    )
                    .map_err(|error| win_failure("register", error))?;
                    registered = true;
                    if !registration_present()? {
                        return Err(failure("registration_not_observed"));
                    }
                    Ok(())
                },
                || {
                    let (sources, summary) = enumerate()?;
                    CONTEXT.with(|slot| {
                        slot.borrow_mut()
                            .as_mut()
                            .expect("context installed")
                            .sources = sources;
                    });
                    Ok(summary)
                },
                || signal_ready(ready_event),
            )?;
            inventory = Some(summary);
            let mut message = MSG::default();
            loop {
                let status = GetMessageW(&mut message, None, 0, 0).0;
                if status == -1 {
                    return Err(win_failure(
                        "message_loop",
                        windows::core::Error::from_thread(),
                    ));
                }
                if status == 0 {
                    break;
                }
                let stop = CONTEXT.with(|slot| {
                    let mut slot = slot.borrow_mut();
                    limits_reached(
                        slot.as_mut().expect("context installed"),
                        Instant::now(),
                        CANCEL.load(Ordering::SeqCst),
                    )
                });
                if stop {
                    break;
                }
                DispatchMessageW(&message);
            }
            Ok(())
        })();
        let mut cleanup = Vec::new();
        if registered {
            if let Err(error) = RegisterRawInputDevices(
                &[RAWINPUTDEVICE {
                    usUsagePage: 1,
                    usUsage: 6,
                    dwFlags: RIDEV_REMOVE,
                    hwndTarget: HWND::default(),
                }],
                size_of::<RAWINPUTDEVICE>() as u32,
            ) {
                cleanup.push(win_failure("unregister", error));
            }
            match registration_present() {
                Ok(false) => (),
                Ok(true) => cleanup.push(failure("registration_still_present")),
                Err(error) => cleanup.push(error),
            }
        }
        if timer {
            if let Err(error) = KillTimer(Some(window), 1) {
                cleanup.push(win_failure("kill_timer", error));
            }
        }
        if let Err(error) = DestroyWindow(window) {
            cleanup.push(win_failure("destroy_window", error));
        }
        if let Err(error) = UnregisterClassW(CLASS, Some(instance)) {
            cleanup.push(win_failure("unregister_class", error));
        }
        if handler {
            if let Err(error) = SetConsoleCtrlHandler(Some(ctrl_handler), false) {
                cleanup.push(win_failure("remove_ctrl_handler", error));
            }
        }
        let mut context = CONTEXT.with(|slot| slot.borrow_mut().take().expect("context installed"));
        context.sample.cancelled |= CANCEL.load(Ordering::SeqCst);
        if let Some(summary) = inventory {
            emit(&summary)?;
        }
        emit(
            &serde_json::json!({ "event": "finished", "outcome": context.sample.outcome(), "sample": context.sample,
            "session_error": session.err().or(context.error), "cleanup_errors": cleanup,
            "scope": "RAWINPUT_only_not_filter_callback_or_fix" }),
        )?;
        if let Some(error) = session.err().or(context.error).or(cleanup.first().copied()) {
            return Err(error);
        }
        Ok(())
    }

    fn fixture(handle: usize, code: u16, flags: u16, vk: u16) -> Vec<u8> {
        let mut bytes = vec![0u8; size_of::<RAWINPUTHEADER>() + size_of::<RAWKEYBOARD>()];
        let header = RAWINPUTHEADER {
            dwType: RIM_TYPEKEYBOARD.0,
            dwSize: bytes.len() as u32,
            hDevice: HANDLE(handle as *mut _),
            wParam: WPARAM(0),
        };
        let keyboard = RAWKEYBOARD {
            MakeCode: code,
            Flags: flags,
            VKey: vk,
            ..Default::default()
        };
        unsafe {
            bytes
                .as_mut_ptr()
                .cast::<RAWINPUTHEADER>()
                .write_unaligned(header);
            bytes
                .as_mut_ptr()
                .add(size_of::<RAWINPUTHEADER>())
                .cast::<RAWKEYBOARD>()
                .write_unaligned(keyboard);
        }
        bytes
    }

    fn selftest() -> Result<()> {
        assert_eq!(size_of::<RAWKEYBOARD>(), 16);
        assert_eq!(
            size_of::<RAWINPUTHEADER>(),
            if size_of::<usize>() == 8 { 24 } else { 16 }
        );
        assert_eq!(offset_of!(RAWINPUT, data), size_of::<RAWINPUTHEADER>());
        assert_eq!(
            size_of::<RAWINPUTDEVICE>(),
            if size_of::<usize>() == 8 { 16 } else { 12 }
        );
        let sources = HashMap::from([
            (1, (Origin::Target, 1)),
            (2, (Origin::Other, 2)),
            (3, (Origin::Unknown, 3)),
        ]);
        assert_eq!(source_for(1, &sources), (Origin::Target, 1));
        assert_eq!(source_for(2, &sources), (Origin::Other, 2));
        assert_eq!(source_for(0, &sources), (Origin::Unknown, 0));
        assert_eq!(source_for(3, &sources), (Origin::Unknown, 3));
        let down = fixture(1, 0x29, 2, 0xc0);
        assert!(decode(&down[..4], Origin::Target, Step::Tv).is_err());
        let mut mismatch = down.clone();
        mismatch[4..8].copy_from_slice(&4u32.to_le_bytes());
        assert!(decode(&mismatch, Origin::Target, Step::Tv).is_err());
        let mut short = down[..size_of::<RAWINPUTHEADER>() + 1].to_vec();
        let short_len = short.len() as u32;
        short[4..8].copy_from_slice(&short_len.to_le_bytes());
        assert!(decode(&short, Origin::Target, Step::Tv).is_err());
        assert!(decode(&down, Origin::Other, Step::Tv)?.is_none());
        assert!(decode(&down, Origin::Unknown, Step::Tv)?.is_none());
        let mut tv = Sample::new(Step::Tv);
        tv.observe(Origin::Other, 2, None);
        tv.observe(Origin::Unknown, 0, None);
        assert_eq!(tv.outcome(), "insufficient");
        assert!(tv.candidate.is_none());
        tv.observe(Origin::Target, 1, decode(&down, Origin::Target, Step::Tv)?);
        assert_eq!(tv.outcome(), "insufficient");
        tv.observe(Origin::Target, 1, decode(&down, Origin::Target, Step::Tv)?);
        let up = fixture(1, 0x29, 3, 0xc0);
        tv.observe(Origin::Target, 1, decode(&up, Origin::Target, Step::Tv)?);
        assert_eq!(tv.outcome(), "candidate_observed");
        assert_eq!(tv.candidate.as_ref().unwrap().repeated_make, 1);
        tv.cancelled = true;
        assert_eq!(tv.outcome(), "cancelled");
        let mut keyboard = Sample::new(Step::Backtick);
        keyboard.observe(
            Origin::Other,
            2,
            decode(&fixture(2, 0x1e, 0, 0x41), Origin::Other, Step::Backtick)?,
        );
        assert!(keyboard.candidate.is_none());
        keyboard.observe(Origin::Target, 1, None);
        assert!(keyboard.candidate.is_none());
        for flags in [0, 1] {
            keyboard.observe(
                Origin::Other,
                2,
                decode(
                    &fixture(2, 0x29, flags, 0xc0),
                    Origin::Other,
                    Step::Backtick,
                )?,
            );
        }
        assert_eq!(keyboard.outcome(), "candidate_observed");
        keyboard.device_changed = true;
        assert_eq!(keyboard.outcome(), "device_changed");
        let serialized = serde_json::to_string(&keyboard).map_err(|_| failure("selftest_json"))?;
        assert!(!serialized.contains("virtual_key"));
        assert!(!serialized.contains("hDevice"));
        let mut orphan = Sample::new(Step::Tv);
        orphan.observe(Origin::Target, 1, decode(&up, Origin::Target, Step::Tv)?);
        assert_eq!(orphan.outcome(), "ambiguous");
        let mut mixed = Sample::new(Step::Backtick);
        mixed.observe(
            Origin::Other,
            2,
            decode(&down, Origin::Other, Step::Backtick)?,
        );
        mixed.observe(
            Origin::Other,
            4,
            decode(&up, Origin::Other, Step::Backtick)?,
        );
        assert_eq!(mixed.outcome(), "ambiguous");
        assert!(utf16(&[1]).is_err());
        assert!(utf16(&[1, 0]).is_err());
        verify_busy_deadline();
        verify_busy_cancel();
        verify_pre_read_bounds();
        verify_blocked_starting();
        verify_starting_failure();
        emit(
            &serde_json::json!({ "event": "selftest", "status": "passed", "registration_calls": 0,
            "coverage": ["official_struct_layout", "boundary_lengths", "source_classification", "paired_edges", "repeat", "privacy", "insufficient", "cancelled", "device_changed", "busy_dispatch_deadline", "busy_dispatch_cancel", "pre_read_deadline_cancel", "blocked_starting_output", "failed_starting_output"] }),
        )
    }

    fn run() -> Result<()> {
        let arguments: Vec<String> = std::env::args().skip(1).collect();
        if arguments == ["--selftest"] {
            return selftest();
        }
        if arguments.len() != 7
            || arguments[0] != "--capture"
            || arguments[1] != "--step"
            || arguments[3] != "--seconds"
            || arguments[5] != "--ready-event"
        {
            return Err(failure(
                "arguments_expected_selftest_or_capture_step_seconds_ready_event",
            ));
        }
        let step = match arguments[2].as_str() {
            "tv" => Step::Tv,
            "backtick" => Step::Backtick,
            _ => return Err(failure("step")),
        };
        let seconds: u64 = arguments[4].parse().map_err(|_| failure("seconds"))?;
        if !(1..=30).contains(&seconds) {
            return Err(failure("seconds_range_1_to_30"));
        }
        start_capture(
            || {
                emit(
                    &serde_json::json!({ "event": "starting", "registration": "not_started", "step": step, "seconds": seconds, "cancel": "Ctrl+C", "candidate_limit": 1 }),
                )
            },
            || unsafe { capture(step, seconds, &arguments[6]) },
        )
    }

    pub fn main() {
        if let Err(error) = run() {
            let _ = emit(&serde_json::json!({ "event": "failed", "error": error }));
            std::process::exit(1);
        }
    }
}

fn main() {
    #[cfg(windows)]
    probe::main();
    #[cfg(not(windows))]
    {
        eprintln!("windows_only");
        std::process::exit(1);
    }
}

//! Device-bound, ordinary-user input channel. No keyboard hooks or synthetic
//! function keys. ABI must match drivers/SayAllInput/protocol.h.
use crate::raw_input::{ButtonEdge, RemoteButton};

const ABI: u32 = 3;
const CANCEL: u32 = 1;
const READY: u32 = 2;
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Event {
    abi: u32,
    sequence: u32,
    buttons: u32,
    flags: u32,
    reason: u32,
    reserved: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Status {
    abi: u32,
    contract: u32,
    active: u32,
    ready: u32,
    sequence: u32,
    cancellations: u32,
    report_count: u32,
    rejected_reports: u32,
    physical_buttons: u32,
    swallowed_buttons: u32,
    observed_report: u32,
}
const BUTTONS: [RemoteButton; 3] = [
    RemoteButton::Back,
    RemoteButton::VolumeUp,
    RemoteButton::VolumeDown,
];

#[derive(Default)]
struct Decoder {
    sequence: Option<u32>,
    buttons: u32,
}
impl Decoder {
    fn accept(&mut self, event: Event) -> Result<(bool, Vec<ButtonEdge>), &'static str> {
        if event.abi != ABI
            || event.buttons & !7 != 0
            || event.flags & !(CANCEL | READY) != 0
            || event.flags == 0
            || event.flags == (CANCEL | READY)
            || event.reserved != 0
            || (event.flags == READY && event.reason != 0)
            || (event.flags == CANCEL && !(1..=7).contains(&event.reason))
        {
            return Err("protocol_invalid");
        }
        if self
            .sequence
            .is_some_and(|seq| event.sequence != seq.wrapping_add(1))
        {
            return Err("sequence_gap");
        }
        self.sequence = Some(event.sequence);
        if event.flags & CANCEL != 0 {
            if event.buttons != 0 {
                return Err("cancel_mask_invalid");
            }
            return Ok((false, self.release()));
        }
        let mut edges = Vec::new();
        // Releases precede presses for a single physical report transition.
        for pressed in [false, true] {
            for (i, button) in BUTTONS.into_iter().enumerate() {
                let bit = 1 << i;
                if self.buttons & bit != event.buttons & bit
                    && (event.buttons & bit != 0) == pressed
                {
                    edges.push(ButtonEdge {
                        button,
                        is_pressed: pressed,
                    });
                }
            }
        }
        self.buttons = event.buttons;
        Ok((true, edges))
    }
    fn release(&mut self) -> Vec<ButtonEdge> {
        let edges = BUTTONS
            .into_iter()
            .enumerate()
            .filter_map(|(i, button)| {
                (self.buttons & (1 << i) != 0).then_some(ButtonEdge {
                    button,
                    is_pressed: false,
                })
            })
            .collect();
        self.buttons = 0;
        edges
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use crate::button_mapping::{ButtonMappingRuntime, EngineMessage};
    use std::ffi::c_void;
    use std::fs::{File, OpenOptions};
    use std::mem::size_of;
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use std::ptr::{null, null_mut};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use std::thread::{self, JoinHandle};
    use std::time::Duration;
    use windows::core::GUID;

    static MAINTENANCE: AtomicBool = AtomicBool::new(false);
    static CLIENT_ACTIVE: AtomicBool = AtomicBool::new(false);
    pub(crate) struct Pause;
    impl Drop for Pause {
        fn drop(&mut self) {
            MAINTENANCE.store(false, Ordering::Release);
        }
    }
    pub(crate) fn pause() -> Result<Pause, &'static str> {
        if MAINTENANCE.swap(true, Ordering::AcqRel) {
            return Err("maintenance_busy");
        }
        let guard = Pause;
        for _ in 0..30 {
            if !CLIENT_ACTIVE.load(Ordering::Acquire) {
                return Ok(guard);
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err("client_close_timeout")
    }
    struct ActiveClient;
    impl Drop for ActiveClient {
        fn drop(&mut self) {
            CLIENT_ACTIVE.store(false, Ordering::Release);
        }
    }

    const INTERFACE: GUID = GUID::from_u128(0x8ab347da4fcb47db96556822457ad573);
    #[repr(C)]
    struct PropertyKey {
        guid: GUID,
        pid: u32,
    }
    const INSTANCE_ID: PropertyKey = PropertyKey {
        guid: GUID::from_u128(0x78c34fc8104a4aca9ea4524d52996e57),
        pid: 256,
    };
    #[repr(C)]
    #[derive(Default)]
    struct Overlapped {
        internal: usize,
        high: usize,
        offset: u32,
        offset_high: u32,
        event: *mut c_void,
    }
    #[link(name = "cfgmgr32")]
    unsafe extern "system" {
        fn CM_Get_Device_Interface_List_SizeW(
            n: *mut u32,
            g: *const GUID,
            id: *const u16,
            flags: u32,
        ) -> u32;
        fn CM_Get_Device_Interface_ListW(
            g: *const GUID,
            id: *const u16,
            list: *mut u16,
            n: u32,
            flags: u32,
        ) -> u32;
        fn CM_Get_Device_Interface_PropertyW(
            path: *const u16,
            key: *const PropertyKey,
            ty: *mut u32,
            buffer: *mut u8,
            size: *mut u32,
            flags: u32,
        ) -> u32;
        fn CM_Locate_DevNodeW(node: *mut u32, id: *const u16, flags: u32) -> u32;
        fn CM_Get_Parent(parent: *mut u32, node: u32, flags: u32) -> u32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateEventW(
            attrs: *const c_void,
            manual: i32,
            initial: i32,
            name: *const u16,
        ) -> *mut c_void;
        fn CloseHandle(handle: *mut c_void) -> i32;
        fn WaitForSingleObject(handle: *mut c_void, ms: u32) -> u32;
        fn DeviceIoControl(
            file: *mut c_void,
            code: u32,
            input: *const c_void,
            input_len: u32,
            output: *mut c_void,
            output_len: u32,
            written: *mut u32,
            overlapped: *mut Overlapped,
        ) -> i32;
        fn GetLastError() -> u32;
        fn CancelIoEx(file: *mut c_void, overlapped: *const Overlapped) -> i32;
        fn GetOverlappedResult(
            file: *mut c_void,
            overlapped: *mut Overlapped,
            written: *mut u32,
            wait: i32,
        ) -> i32;
        fn WTSGetActiveConsoleSessionId() -> u32;
        fn ProcessIdToSessionId(pid: u32, session: *mut u32) -> i32;
    }
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }
    fn device_node(path: &str) -> Result<u32, &'static str> {
        let path = wide(path);
        let mut ty = 0;
        let mut bytes = 0;
        unsafe {
            CM_Get_Device_Interface_PropertyW(
                path.as_ptr(),
                &INSTANCE_ID,
                &mut ty,
                null_mut(),
                &mut bytes,
                0,
            );
            if bytes == 0 || bytes > 65536 {
                return Err("device_identity_unavailable");
            }
            let mut id = vec![0u16; bytes as usize / 2 + 1];
            if CM_Get_Device_Interface_PropertyW(
                path.as_ptr(),
                &INSTANCE_ID,
                &mut ty,
                id.as_mut_ptr().cast(),
                &mut bytes,
                0,
            ) != 0
                || ty != 0x12
            {
                return Err("device_identity_unavailable");
            }
            let mut node = 0;
            if CM_Locate_DevNodeW(&mut node, id.as_ptr(), 0) != 0 {
                return Err("device_identity_unavailable");
            }
            Ok(node)
        }
    }
    fn bound_interface(selected: &str) -> Result<String, &'static str> {
        let selected = device_node(selected)?;
        let mut n = 0;
        unsafe {
            if CM_Get_Device_Interface_List_SizeW(&mut n, &INTERFACE, null(), 0) != 0 || n > 65536 {
                return Err("enumeration_failed");
            }
            let mut list = vec![0u16; n as usize];
            if CM_Get_Device_Interface_ListW(&INTERFACE, null(), list.as_mut_ptr(), n, 0) != 0 {
                return Err("enumeration_failed");
            }
            let mut matches = Vec::new();
            for path in list.split(|c| *c == 0).filter(|s| !s.is_empty()) {
                let path = String::from_utf16(path).map_err(|_| "interface_invalid")?;
                let child = device_node(&path)?;
                let mut parent = 0;
                if CM_Get_Parent(&mut parent, child, 0) == 0 && parent == selected {
                    matches.push(path);
                }
            }
            if matches.len() != 1 {
                return Err(if matches.is_empty() {
                    "bound_channel_missing"
                } else {
                    "bound_channel_ambiguous"
                });
            }
            Ok(matches.remove(0))
        }
    }
    fn channel_paths() -> Result<Vec<String>, &'static str> {
        let mut n = 0;
        unsafe {
            if CM_Get_Device_Interface_List_SizeW(&mut n, &INTERFACE, null(), 0) != 0 || n > 65536 {
                return Err("enumeration_failed");
            }
            let mut list = vec![0; n as usize];
            if CM_Get_Device_Interface_ListW(&INTERFACE, null(), list.as_mut_ptr(), n, 0) != 0 {
                return Err("enumeration_failed");
            }
            list.split(|c| *c == 0)
                .filter(|p| !p.is_empty())
                .map(|p| String::from_utf16(p).map_err(|_| "interface_invalid"))
                .collect()
        }
    }
    /// Passive inspection only. Holding exclusive handles prevents new CLAIMs
    /// throughout maintenance. Existing clients or unknown release state block.
    pub(crate) fn maintenance_channels() -> Result<Vec<File>, &'static str> {
        let mut held = Vec::new();
        for path in channel_paths()? {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .share_mode(0)
                .custom_flags(0x40000000)
                .open(path)
                .map_err(|_| "channel_in_use")?;
            let status: Status = ioctl(&file, 0x226000, None, &AtomicBool::new(false))?;
            if status.abi != ABI || status.contract != 1 || status.observed_report != 1 {
                return Err("release_state_unknown");
            }
            if status.active != 0 || status.physical_buttons != 0 || status.swallowed_buttons != 0 {
                return Err("buttons_not_released");
            }
            let _: Status = ioctl(&file, 0x22A010, None, &AtomicBool::new(false))?;
            held.push(file);
        }
        Ok(held)
    }
    pub(crate) fn inspect_channels() -> Result<(usize, bool), &'static str> {
        let paths = channel_paths()?;
        let mut valid = !paths.is_empty();
        for path in &paths {
            let file = OpenOptions::new()
                .read(true)
                .share_mode(3)
                .custom_flags(0x40000000)
                .open(path)
                .map_err(|_| "channel_in_use")?;
            let status: Status = ioctl(&file, 0x226000, None, &AtomicBool::new(false))?;
            valid &= status.abi == ABI && status.contract == 1;
        }
        Ok((paths.len(), valid))
    }
    struct Request<T> {
        overlapped: Overlapped,
        output: T,
        input: u32,
    }
    impl<T> Drop for Request<T> {
        fn drop(&mut self) {
            if !self.overlapped.event.is_null() {
                unsafe {
                    CloseHandle(self.overlapped.event);
                }
            }
        }
    }
    fn ioctl<T: Default>(
        file: &File,
        code: u32,
        input: Option<u32>,
        stop: &AtomicBool,
    ) -> Result<T, &'static str> {
        let mut request = Box::new(Request {
            overlapped: Overlapped::default(),
            output: T::default(),
            input: input.unwrap_or(0),
        });
        unsafe {
            request.overlapped.event = CreateEventW(null(), 1, 0, null());
            if request.overlapped.event.is_null() {
                return Err("event_failed");
            }
            let mut written = 0;
            let ok = DeviceIoControl(
                file.as_raw_handle(),
                code,
                if input.is_some() {
                    (&request.input as *const u32).cast()
                } else {
                    null()
                },
                if input.is_some() { 4 } else { 0 },
                (&mut request.output as *mut T).cast(),
                size_of::<T>() as u32,
                &mut written,
                &mut request.overlapped,
            );
            if ok == 0 {
                let error = GetLastError();
                if error == 170 {
                    return Err("read_busy");
                }
                if error != 997 {
                    return Err("ioctl_rejected");
                }
            }
            if ok == 0 {
                let mut signaled = false;
                for _ in 0..10 {
                    if WaitForSingleObject(request.overlapped.event, 100) == 0 {
                        signaled = true;
                        break;
                    }
                    if stop.load(Ordering::Acquire) {
                        break;
                    }
                }
                if !signaled {
                    CancelIoEx(file.as_raw_handle(), &request.overlapped);
                    if WaitForSingleObject(request.overlapped.event, 500) != 0 {
                        // A broken kernel must not retain a pointer into freed
                        // user memory. Retain this one request and fail closed.
                        Box::leak(request);
                        return Err("cancellation_unconfirmed");
                    }
                    // Completion can win the cancellation race. Read its
                    // actual result below so a delivered edge is never lost.
                }
                if GetOverlappedResult(
                    file.as_raw_handle(),
                    &mut request.overlapped,
                    &mut written,
                    0,
                ) == 0
                {
                    return Err(if GetLastError() == 995 {
                        "read_timeout"
                    } else {
                        "ioctl_failed"
                    });
                }
            }
            if written as usize != size_of::<T>() {
                return Err("reply_size_invalid");
            }
        }
        Ok(std::mem::take(&mut request.output))
    }
    fn connection(
        selected: &str,
        stop: &AtomicBool,
        mapping: &ButtonMappingRuntime,
    ) -> Result<(), &'static str> {
        CLIENT_ACTIVE.store(true, Ordering::Release);
        let _active = ActiveClient;
        if MAINTENANCE.load(Ordering::Acquire) {
            return Err("maintenance_active");
        }
        fn active_session() -> bool {
            unsafe {
                let mut session = 0;
                ProcessIdToSessionId(std::process::id(), &mut session) != 0
                    && session == WTSGetActiveConsoleSessionId()
            }
        }
        if !active_session() {
            return Err("interactive_session_inactive");
        }
        let desired = mapping.requested_input_enhancement();
        if desired == 0 {
            return Err("mapping_not_requested");
        }
        let path = bound_interface(selected)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(3)
            .custom_flags(0x40000000)
            .open(&path)
            .map_err(|_| "channel_open_failed")?;
        let status: Status = ioctl(&file, 0x226000, None, stop)?;
        if status.abi != ABI || status.contract != 1 {
            return Err("report_contract_unverified");
        }
        let claimed: Status = ioctl(&file, 0x22A004, Some(desired), stop)?;
        if claimed.abi != ABI || claimed.active != 1 {
            return Err("claim_rejected");
        }
        crate::gatt_note("input_driver phase=claimed binding=selected_hid_parent report_contract=1 privilege=ordinary_user".to_owned());
        let mut decoder = Decoder::default();
        let result = (|| {
            loop {
                if stop.load(Ordering::Acquire)
                    || MAINTENANCE.load(Ordering::Acquire)
                    || !active_session()
                    || mapping.requested_input_enhancement() != desired
                {
                    return Ok(());
                }
                match ioctl::<Event>(&file, 0x226008, None, stop) {
                    Err("read_timeout") => continue, // Cancelled pending read renews the next lease.
                    Err("read_busy") => {
                        thread::yield_now();
                        continue;
                    }
                    Err(reason) => return Err(reason),
                    Ok(event) => {
                        let (ready, edges) = decoder.accept(event).map_err(|reason| {
                            crate::gatt_note(format!("input_driver phase=decode_rejected reason={reason} abi={} sequence={} flags={} cancel_reason={}",event.abi,event.sequence,event.flags,event.reason));
                            reason
                        })?;
                        mapping.set_input_enhancement(ready);
                        let reason = match event.reason {
                            0 => "none",
                            1 => "released",
                            2 => "client_cleanup",
                            3 => "lease_expired",
                            4 => "queue_overflow",
                            5 => "report_rejected",
                            6 => "power_transition",
                            7 => "waiting_all_up",
                            _ => "unknown",
                        };
                        crate::gatt_note(format!("input_driver phase=event sequence={} ready={ready} edges={} flags={} reason={reason}",event.sequence,edges.len(),event.flags));
                        for edge in edges {
                            let _ = mapping.sender().send(EngineMessage::DriverEdge(edge));
                        }
                        if !ready {
                            let current: Status = ioctl(&file, 0x226000, None, stop)?;
                            if current.active != 1 {
                                return Err("driver_cancelled");
                            }
                        }
                    }
                }
            }
        })();
        mapping.set_input_enhancement(false);
        for edge in decoder.release() {
            let _ = mapping.sender().send(EngineMessage::DriverEdge(edge));
        }
        let _: Result<Status, _> = ioctl(&file, 0x22A00C, None, stop);
        result
    }
    pub(crate) fn start(
        selected: String,
        stop: Arc<AtomicBool>,
        mapping: Arc<ButtonMappingRuntime>,
    ) -> Option<JoinHandle<()>> {
        thread::Builder::new()
            .name("sayall-input-driver".into())
            .spawn(move || {
                let mut last = None;
                while !stop.load(Ordering::Acquire) {
                    let outcome = connection(&selected, &stop, &mapping);
                    if outcome != last.unwrap_or(Ok(())) {
                        crate::gatt_note(format!(
                            "input_driver phase=channel terminal_result={} reason={}",
                            if outcome.is_ok() {
                                "closed"
                            } else {
                                "unavailable"
                            },
                            outcome.err().unwrap_or("stopped")
                        ));
                    }
                    last = Some(outcome);
                    // An unconfirmed kernel cancellation retains one request
                    // buffer for safety. Do not allocate more on reconnect.
                    if outcome == Err("cancellation_unconfirmed") {
                        break;
                    }
                    for _ in 0..10 {
                        if stop.load(Ordering::Acquire) {
                            break;
                        }
                        thread::sleep(Duration::from_millis(100));
                    }
                }
                mapping.set_input_enhancement(false);
            })
            .ok()
    }
}
#[cfg(windows)]
pub(crate) use platform::start;
#[cfg(windows)]
pub(crate) use platform::{inspect_channels, maintenance_channels, pause};

#[cfg(test)]
mod tests {
    use super::*;
    fn event(sequence: u32, buttons: u32, flags: u32) -> Event {
        Event {
            abi: ABI,
            sequence,
            buttons,
            flags,
            reason: if flags == CANCEL { 1 } else { 0 },
            reserved: 0,
        }
    }
    #[test]
    fn driver_protocol_fast_edges_and_cancel_are_paired() {
        let mut d = Decoder::default();
        assert!(d.accept(event(1, 0, READY)).unwrap().1.is_empty());
        let down = d.accept(event(2, 7, READY)).unwrap().1;
        assert_eq!(down.len(), 3);
        assert!(down.iter().all(|e| e.is_pressed));
        assert!(d.accept(event(3, 7, READY)).unwrap().1.is_empty());
        let (ready, up) = d.accept(event(4, 0, CANCEL)).unwrap();
        assert!(!ready);
        assert_eq!(up.len(), 3);
        assert!(up.iter().all(|e| !e.is_pressed));
        assert!(d.release().is_empty());
    }
    #[test]
    fn driver_protocol_rejects_gaps_and_untrusted_abi() {
        let mut d = Decoder::default();
        d.accept(event(10, 1, READY)).unwrap();
        assert!(d.accept(event(12, 0, READY)).is_err());
        assert_eq!(d.release().len(), 1);
        assert!(Decoder::default()
            .accept(Event {
                abi: 99,
                ..event(1, 0, READY)
            })
            .is_err());
        assert!(Decoder::default().accept(event(1, 8, READY)).is_err());
        assert!(Decoder::default().accept(event(1, 1, CANCEL)).is_err());
    }
}

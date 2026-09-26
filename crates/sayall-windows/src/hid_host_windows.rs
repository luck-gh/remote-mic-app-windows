//! Ordinary-user client for the explicitly started, fixed RC003 HID Helper.
use super::{cancel_mapping, Edges, Report};
use crate::button_mapping::{ButtonMappingRuntime, EngineMessage};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::ffi::c_void;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use windows::core::{w, GUID, PCWSTR};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::{
    ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS,
    SHELLEXECUTEINFOW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

static REQUESTED: AtomicBool = AtomicBool::new(false);
static LISTENING: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static STATUS: Mutex<String> = Mutex::new(String::new());
const HELPER_SHA: Option<&str> = option_env!("SAYALL_HID_HOST_HELPER_SHA256");
type Handle = *mut c_void;
#[repr(C)]
struct SecurityAttributes {
    length: u32,
    descriptor: Handle,
    inherit: i32,
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateNamedPipeW(
        name: *const u16,
        open: u32,
        mode: u32,
        instances: u32,
        out: u32,
        input: u32,
        timeout: u32,
        security: *const SecurityAttributes,
    ) -> Handle;
    fn ConnectNamedPipe(pipe: Handle, overlapped: Handle) -> i32;
    fn PeekNamedPipe(
        pipe: Handle,
        buffer: Handle,
        size: u32,
        read: *mut u32,
        available: *mut u32,
        remaining: *mut u32,
    ) -> i32;
    fn GetNamedPipeClientProcessId(pipe: Handle, pid: *mut u32) -> i32;
    fn GetProcessId(process: Handle) -> u32;
    fn WaitForSingleObject(handle: Handle, timeout: u32) -> u32;
    fn CloseHandle(handle: Handle) -> i32;
    fn LocalFree(memory: Handle) -> Handle;
}
#[link(name = "advapi32")]
unsafe extern "system" {
    fn ConvertStringSecurityDescriptorToSecurityDescriptorW(
        text: *const u16,
        revision: u32,
        descriptor: *mut Handle,
        size: *mut u32,
    ) -> i32;
}
struct Process(Handle);
unsafe impl Send for Process {}
impl Drop for Process {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
fn status(value: impl Into<String>) {
    *STATUS.lock().unwrap_or_else(|p| p.into_inner()) = value.into();
}
pub fn current_status() -> String {
    let value = STATUS.lock().unwrap_or_else(|p| p.into_inner());
    if value.is_empty() {
        "未启动".into()
    } else {
        value.clone()
    }
}
/// A saved, explicit opt-in queues one ordinary runas request. No retry after UAC cancellation.
pub fn restore_on_start() {
    if ACTIVE.load(Ordering::Acquire) {
        return;
    }
    if HELPER_SHA.is_none() {
        status("异常：此安装包未包含三键增强；请安装包含 Helper 的本地包");
        return;
    }
    REQUESTED.store(true, Ordering::Release);
    status("等待遥控器：连接就绪后请求管理员授权");
    log("restore_requested opt_in=true retry_after_cancel=false");
}
pub fn cancel_pending_start() {
    if REQUESTED.swap(false, Ordering::AcqRel) && !ACTIVE.load(Ordering::Acquire) {
        status("未启动：已取消待执行的自动恢复");
    }
    log("restore_disabled current_session_unchanged=true");
}
pub fn request_start() -> Result<String, String> {
    if HELPER_SHA.is_none() {
        return Err("当前安装包未包含三键增强。".into());
    }
    if !LISTENING.load(Ordering::Acquire) {
        return Err("请等待遥控器连接和按键监听就绪。".into());
    }
    if ACTIVE.load(Ordering::Acquire) {
        return Ok(current_status());
    }
    REQUESTED.store(true, Ordering::Release);
    status("已请求启动；请在系统窗口确认管理员授权");
    Ok(current_status())
}
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
fn log(reason: &str) {
    crate::gatt_note(format!("hid_host phase={reason}"));
}
fn pipe(nonce: &str) -> Result<File, &'static str> {
    let name = wide(&format!(
        r"\\.\pipe\SayAllHidHost-{}-{nonce}",
        std::process::id()
    ));
    let sddl = wide("D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)");
    let mut descriptor = std::ptr::null_mut();
    unsafe {
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            std::ptr::null_mut(),
        ) == 0
        {
            return Err("pipe_acl");
        }
        let attributes = SecurityAttributes {
            length: std::mem::size_of::<SecurityAttributes>() as u32,
            descriptor,
            inherit: 0,
        };
        // Message mode, local clients only, nonblocking; a single exact Helper peer.
        let handle = CreateNamedPipeW(
            name.as_ptr(),
            3 | 0x80000,
            4 | 2 | 1 | 8,
            1,
            65536,
            8192,
            0,
            &attributes,
        );
        LocalFree(descriptor);
        if handle as isize == -1 {
            return Err("pipe_create");
        }
        Ok(File::from_raw_handle(handle))
    }
}
fn launch(nonce: &str) -> Result<(Process, File), &'static str> {
    let expected = HELPER_SHA.ok_or("helper_not_packaged")?;
    let executable = std::env::current_exe().map_err(|_| "application_path")?;
    let parent = executable.parent().ok_or("application_path")?;
    let path = parent.join("sayall-hid-host-helper.exe");
    let mut file = OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&path)
        .map_err(|_| "helper_open")?;
    let mut hash = Sha256::new();
    let mut bytes = [0u8; 65536];
    loop {
        let count = file.read(&mut bytes).map_err(|_| "helper_read")?;
        if count == 0 {
            break;
        }
        hash.update(&bytes[..count]);
    }
    if format!("{:x}", hash.finalize()) != expected {
        return Err("helper_hash");
    }
    let path = wide(path.to_str().ok_or("helper_path")?);
    let directory = wide(parent.to_str().ok_or("helper_path")?);
    let arguments = wide(&format!("{} {nonce}", std::process::id()));
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)
            .ok()
            .map_err(|_| "helper_apartment")?;
    }
    let mut launch = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_FLAG_NO_UI | SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
        lpVerb: w!("runas"),
        lpFile: PCWSTR(path.as_ptr()),
        lpParameters: PCWSTR(arguments.as_ptr()),
        lpDirectory: PCWSTR(directory.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    let outcome = unsafe { ShellExecuteExW(&mut launch) };
    unsafe {
        CoUninitialize();
    }
    outcome.map_err(|error| {
        if error.code().0 as u32 == 0x800704c7 {
            "uac_cancelled"
        } else {
            "helper_launch_failed"
        }
    })?;
    if launch.hProcess.is_invalid() {
        return Err("helper_process");
    }
    Ok((Process(launch.hProcess.0), file))
}
fn send(pipe: &mut File, value: serde_json::Value) -> Result<(), &'static str> {
    let bytes = serde_json::to_vec(&value).map_err(|_| "command_encode")?;
    if pipe.write(&bytes).map_err(|_| "command_write")? != bytes.len() {
        return Err("command_short");
    }
    Ok(())
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Event {
    kind: String,
    #[serde(default)]
    generation: u64,
    #[serde(default)]
    sequence: u64,
    #[serde(default)]
    ready: bool,
    #[serde(default)]
    buttons: u8,
    #[serde(default)]
    physical: u8,
    #[serde(default)]
    suppressed: u8,
    #[serde(default)]
    all_up: bool,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    length: usize,
    #[serde(default)]
    configuration: u64,
    #[serde(default)]
    mask: u32,
    #[serde(default)]
    basis: String,
    #[serde(default)]
    reconnected_first: bool,
    #[serde(default)]
    prior_released: bool,
    #[serde(default)]
    native_gap: bool,
    #[serde(default)]
    coverage_known: bool,
    #[serde(default)]
    raw_released: bool,
    #[serde(default)]
    operation: String,
    #[serde(default)]
    error_class: String,
    #[serde(default)]
    code_line: u32,
    #[serde(default)]
    cleanup_errors: u32,
    property_status: Option<i32>,
    property_type: Option<u32>,
    property_bytes: Option<u32>,
}
fn receive(pipe: &mut File) -> Result<Option<Event>, &'static str> {
    // std::fs::File translates an empty PIPE_NOWAIT read to Ok(0), just
    // like a closed peer. Query the actual pipe state before consuming data.
    // This handle has one reader on this worker; Peek and Read cannot race
    // another local consumer or an outstanding blocking operation.
    let mut available = 0;
    if unsafe {
        PeekNamedPipe(
            pipe.as_raw_handle(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            &mut available,
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err("helper_closed");
    }
    if available == 0 {
        return Ok(None);
    }
    let mut bytes = [0u8; 4096];
    match pipe.read(&mut bytes) {
        Ok(0) => Err("helper_closed"),
        Ok(count) => serde_json::from_slice(&bytes[..count])
            .map(Some)
            .map_err(|_| "event_contract"),
        Err(error) if matches!(error.raw_os_error(), Some(232 | 536)) => Ok(None),
        Err(_) => Err("event_read"),
    }
}
fn run(
    selected: &str,
    stop: &AtomicBool,
    mapping: &ButtonMappingRuntime,
) -> Result<(), &'static str> {
    let nonce = format!("{:032x}", GUID::new().map_err(|_| "nonce")?.to_u128());
    let mut channel = pipe(&nonce)?;
    let (process, _locked_helper) = launch(&nonce)?;
    let expected_pid = unsafe { GetProcessId(process.0) };
    let started = Instant::now();
    loop {
        unsafe {
            ConnectNamedPipe(channel.as_raw_handle(), std::ptr::null_mut());
        }
        let mut pid = 0;
        if unsafe { GetNamedPipeClientProcessId(channel.as_raw_handle(), &mut pid) } != 0 {
            if pid != expected_pid {
                return Err("pipe_peer");
            }
            break;
        }
        if stop.load(Ordering::Acquire) || unsafe { WaitForSingleObject(process.0, 0) } != 258 {
            return Err("helper_start_failed");
        }
        if started.elapsed() > Duration::from_secs(15) {
            return Err("helper_connect_timeout");
        }
        thread::sleep(Duration::from_millis(10));
    }
    let mut desired = mapping.requested_host_enhancement();
    send(
        &mut channel,
        serde_json::json!({"kind":"init","selected":selected,"mask":desired}),
    )?;
    log("peer_verified privilege=ordinary_user");
    let mut edges = Edges::default();
    let mut generation = 0;
    let mut sequence = 0;
    let mut heartbeat = Instant::now();
    let mut edge_sequence = 0;
    let mut configuration = 0;
    let mut configured = false;
    let outcome = (|| {
        loop {
            if stop.load(Ordering::Acquire) {
                let _ = send(&mut channel, serde_json::json!({"kind":"stop"}));
                return Ok(());
            }
            let next = mapping.requested_host_enhancement();
            if next != desired {
                cancel_mapping(mapping, &mut edges);
                desired = next;
                configuration += 1;
                configured = false;
                let raw_released = mapping.host_raw_released(Duration::from_secs(1));
                crate::gatt_note(format!("hid_host phase=configuration_barrier configuration={configuration} raw_release_known={} raw_released={}", raw_released.is_some(), raw_released == Some(true)));
                send(
                    &mut channel,
                    serde_json::json!({"kind":"configure","mask":desired,"configuration":configuration,"preserveReleased":raw_released == Some(true)}),
                )?;
            }
            if heartbeat.elapsed() >= Duration::from_millis(500) {
                send(&mut channel, serde_json::json!({"kind":"heartbeat"}))?;
                heartbeat = Instant::now();
            }
            while let Some(event) = receive(&mut channel)? {
                if event.kind == "cancel" && event.generation == 0 {
                    cancel_mapping(mapping, &mut edges);
                    let _ = mapping.sender().send(EngineMessage::HidObservation(0));
                    generation = 0;
                    sequence = 0;
                    status("增强通道已断开，正在等待恢复");
                    continue;
                }
                if event.generation != generation {
                    cancel_mapping(mapping, &mut edges);
                    generation = event.generation;
                    sequence = 0;
                }
                if event.sequence != sequence + 1 {
                    return Err("event_sequence");
                }
                sequence = event.sequence;
                match event.kind.as_str() {
                    "bound" => {
                        let _ = mapping.sender().send(EngineMessage::HidObservation(0));
                        configured = false;
                        status("增强通道已建立，等待确认释放状态");
                    }
                    "configured" => {
                        configured = event.configuration == configuration && event.mask == desired;
                        crate::gatt_note(format!("hid_host phase=configured generation={generation} configuration={} mask={} accepted={configured}",event.configuration,event.mask));
                        status("增强通道已建立，等待确认释放状态");
                    }
                    "state" => {
                        if !configured || event.configuration != configuration {
                            continue;
                        }
                        if event.buttons & !31 != 0
                            || event.physical & !31 != 0
                            || event.suppressed & !31 != 0
                            || event.buttons & !event.suppressed != 0
                            || (!event.ready && event.buttons != 0)
                        {
                            return Err("event_mask");
                        }
                        // Already authenticated peer, current configuration/sequence and
                        // validated report. Observation does not claim or map any usage.
                        if event.basis != "configuration_reuse" {
                            let _ = mapping
                                .sender()
                                .send(EngineMessage::HidObservation(event.physical));
                        }
                        mapping.set_input_enhancement(event.ready);
                        // The wire has non-state messages too; the edge decoder has its own contiguous counter.
                        let transitions = edges.accept_verified(
                            sequence_for_edges(&mut edge_sequence),
                            Report {
                                buttons: event.buttons,
                                all_released: event.all_up,
                            },
                            event.ready,
                        )?;
                        for edge in transitions {
                            let _ = mapping.sender().send(EngineMessage::DriverEdge(edge));
                        }
                        status(if event.ready {
                            "已就绪：三键增强已接管"
                        } else if desired == 0 && event.all_up {
                            "等待映射：当前窗口未配置增强按键动作"
                        } else {
                            "等待遥控器释放后接管"
                        });
                        let report_basis = if event.basis == "configuration_reuse" {
                            "configuration_reuse"
                        } else {
                            "request"
                        };
                        crate::gatt_note(format!("hid_host phase=state generation={generation} sequence={sequence} configuration={configuration} mask={desired} source=selected_instance report_basis={report_basis} ready={} all_up={} physical={} suppressed={} mapped={} length={} reconnect_first={} prior_released={} native_gap={} coverage_known={} raw_released={}",event.ready,event.all_up,event.physical,event.suppressed,event.buttons,event.length,event.reconnected_first,event.prior_released,event.native_gap,event.coverage_known,event.raw_released));
                    }
                    "cancel" => {
                        let _ = mapping.sender().send(EngineMessage::HidObservation(0));
                        cancel_mapping(mapping, &mut edges);
                        status("增强已取消，等待重新接管");
                        log("mapping_cancelled");
                    }
                    "rejected" => {
                        if !edges.ready() {
                            status("来源或报告校验未通过，原生输入保持放行");
                        }
                        if matches!(event.reason.as_str(), "control_failure" | "cleanup_failure") {
                            let operation = match event.operation.as_str() {
                                "device_reference_release"
                                | "hardware_reference_release"
                                | "device_cancel"
                                | "reference_release"
                                | "listener_detach" => event.operation.as_str(),
                                _ => "command",
                            };
                            let error_class = match event.error_class.as_str() {
                                "access_violation" | "TypeError" | "ReferenceError"
                                | "RangeError" | "SyntaxError" => event.error_class.as_str(),
                                _ => "runtime_error",
                            };
                            crate::gatt_note(format!("hid_host phase=rejected reason={} operation={operation} class={error_class} code_line={} native=passthrough_new_holds",event.reason,event.code_line.min(10000)));
                        }
                        if matches!(
                            event.reason.as_str(),
                            "queue_missing"
                                | "device_missing"
                                | "source_property_status"
                                | "source_property_buffer_small"
                                | "source_property_buffer_overflow"
                                | "source_property_length"
                                | "source_property_termination"
                                | "source_property_embedded_null"
                                | "source_mismatch"
                                | "source_device_changed"
                                | "source_retiring"
                                | "report_length"
                                | "output_buffer"
                                | "report_contract"
                        ) {
                            if let (Some(status), Some(kind), Some(bytes)) = (
                                event.property_status,
                                event.property_type,
                                event.property_bytes,
                            ) {
                                crate::gatt_note(format!(
                                    "hid_host phase=rejected reason={} native=passthrough property_status=0x{:08x} property_type={kind} property_bytes={bytes} source_relation=unverified",
                                    event.reason, status as u32
                                ));
                            } else {
                                crate::gatt_note(format!(
                                    "hid_host phase=rejected reason={} native=passthrough",
                                    event.reason
                                ));
                            }
                        }
                    }
                    "reference_released" if event.basis == "release_hardware" => {
                        crate::gatt_note(
                            "hid_host phase=reference_released basis=release_hardware held=0"
                                .to_owned(),
                        );
                    }
                    "stopped" => {
                        crate::gatt_note(format!(
                            "hid_host phase=script_stopped cleanup_errors={}",
                            event.cleanup_errors
                        ));
                        return if event.cleanup_errors == 0 {
                            Ok(())
                        } else {
                            Err("helper_cleanup_failed")
                        };
                    }
                    _ => return Err("event_kind"),
                }
            }
            if unsafe { WaitForSingleObject(process.0, 0) } != 258 {
                return Err("helper_exited");
            }
            thread::sleep(Duration::from_millis(10));
        }
    })();
    let _ = mapping.sender().send(EngineMessage::HidObservation(0));
    cancel_mapping(mapping, &mut edges);
    let _ = send(&mut channel, serde_json::json!({"kind":"stop"}));
    log("client_released helper_drains_physical_holds=true");
    outcome
}
fn sequence_for_edges(sequence: &mut u64) -> u64 {
    *sequence += 1;
    *sequence
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connected_pipe() -> (File, File) {
        let nonce = format!("{:032x}", GUID::new().unwrap().to_u128());
        let server = pipe(&nonce).unwrap();
        let client = OpenOptions::new()
            .read(true)
            .write(true)
            .open(format!(
                r"\\.\pipe\SayAllHidHost-{}-{nonce}",
                std::process::id()
            ))
            .unwrap();
        unsafe {
            ConnectNamedPipe(server.as_raw_handle(), std::ptr::null_mut());
        }
        (server, client)
    }

    #[test]
    fn named_pipe_temporary_empty_message_and_normal_close_are_distinct() {
        let (mut server, mut client) = connected_pipe();
        assert!(receive(&mut server).unwrap().is_none());
        client
            .write_all(br#"{"kind":"bound","sequence":1}"#)
            .unwrap();
        let event = receive(&mut server).unwrap().unwrap();
        assert_eq!(event.kind, "bound");
        assert_eq!(event.sequence, 1);
        assert!(receive(&mut server).unwrap().is_none());
        client
            .write_all(br#"{"kind":"rejected","sequence":2,"reason":"source_property_status","propertyStatus":-1073741811,"propertyType":0,"propertyBytes":0}"#)
            .unwrap();
        let rejected = receive(&mut server).unwrap().unwrap();
        assert_eq!(rejected.property_status, Some(-1073741811));
        assert_eq!(rejected.property_type, Some(0));
        assert_eq!(rejected.property_bytes, Some(0));
        drop(client);
        assert!(receive(&mut server).is_err());
        assert!(send(&mut server, serde_json::json!({"kind":"stop"})).is_err());
    }

    #[test]
    fn named_pipe_nonblocking_backpressure_is_never_reported_as_a_sent_command() {
        let (mut server, _client) = connected_pipe();
        assert!(send(&mut server, serde_json::json!({"kind":"stop"})).is_ok());
        let result = send(
            &mut server,
            serde_json::json!({"padding":"x".repeat(131072)}),
        );
        assert!(matches!(result, Err("command_short" | "command_write")));
    }
}
pub(crate) fn start(
    selected: String,
    stop: Arc<AtomicBool>,
    mapping: Arc<ButtonMappingRuntime>,
) -> Option<JoinHandle<()>> {
    thread::Builder::new()
        .name("sayall-hid-host".into())
        .spawn(move || {
            LISTENING.store(true, Ordering::Release);
            while !stop.load(Ordering::Acquire) {
                if REQUESTED.swap(false, Ordering::AcqRel) {
                    ACTIVE.store(true, Ordering::Release);
                    status("启动中：请在系统窗口确认管理员授权");
                    let outcome = run(&selected, &stop, &mapping);
                    if let Err(reason) = outcome {
                        crate::gatt_note(format!("hid_host phase=unavailable reason={reason}"));
                        let description = match reason {
                            "uac_cancelled" => "未启动：已取消管理员授权；可点击启动重试",
                            "helper_hash" | "helper_open" | "helper_not_packaged" => {
                                "异常：Helper 缺失或校验失败；请重新安装完整本地包"
                            }
                            "helper_launch_failed" => "异常：管理员启动失败；可点击启动重试",
                            "helper_connect_timeout" | "helper_start_failed" => {
                                "异常：Helper 未完成握手；请查看增强诊断后手动重试"
                            }
                            _ => "异常：增强通道中止；具体阶段已记录，可点击启动重试",
                        };
                        status(format!("{description}（{reason}）"));
                    }
                    ACTIVE.store(false, Ordering::Release);
                }
                thread::sleep(Duration::from_millis(50));
            }
            LISTENING.store(false, Ordering::Release);
        })
        .ok()
}

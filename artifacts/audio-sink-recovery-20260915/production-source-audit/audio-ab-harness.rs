// Standalone optimized experiment. Not a Cargo workspace target or CI test.
// The two module files are byte-for-byte verified production source snapshots.
#![allow(dead_code)]
pub use sayall_windows::{AudioEndpoint, AudioPhase, AudioSnapshot, PlatformError, is_virtual_cable_output_name};
use std::{sync::atomic::{AtomicU8, Ordering}, thread, time::{Duration, Instant}};
static VARIANT: AtomicU8 = AtomicU8::new(0);
mod ble {
    pub fn gatt_note(message: String) {
        println!("ab_source variant={} {message}", super::VARIANT.load(super::Ordering::Relaxed));
    }
}
#[path = "old-production-audio.rs"] mod old;
#[path = "installed-recovery-audio.rs"] mod new;
enum Runtime { Old(old::AudioRuntime), New(new::AudioRuntime) }
impl Runtime {
    fn restore(&self, id: String, name: String) -> Result<AudioSnapshot, PlatformError> {
        match self { Self::Old(a) => a.restore_endpoint(id, name), Self::New(a) => a.restore_endpoint(id, name) }
    }
    fn begin(&self) -> Result<AudioSnapshot, PlatformError> {
        match self { Self::Old(a) => a.begin_session(1), Self::New(a) => a.begin_session(1, None) }
    }
    fn send(&self, samples: Vec<i16>) -> Result<(), PlatformError> {
        match self { Self::Old(a) => a.enqueue_samples(1, samples), Self::New(a) => a.enqueue_samples(1, samples) }
    }
    fn snapshot(&self) -> AudioSnapshot { match self { Self::Old(a) => a.snapshot(), Self::New(a) => a.snapshot() } }
    fn finish(&self) -> Result<AudioSnapshot, PlatformError> { match self { Self::Old(a) => a.finish_session(1), Self::New(a) => a.finish_session(1) } }
    fn interrupt(&self) -> Result<AudioSnapshot, PlatformError> { match self { Self::Old(a) => a.interrupt_session(), Self::New(a) => a.interrupt_session() } }
}
fn run(variant: u8, id: &str, name: &str) -> bool {
    VARIANT.store(variant, Ordering::Relaxed);
    capture_context();
    let consumer = if std::env::args().any(|a| a == "--with-consumer") {
        match Consumer::start(id, name) { Ok(c) => Some(c), Err(stage) => { println!("ab_result variant={variant} consumer_setup=false stage={stage}"); return false; } }
    } else { None };
    let runtime = if variant == 0 { Runtime::Old(old::AudioRuntime::new()) } else { Runtime::New(new::AudioRuntime::new()) };
    if runtime.restore(id.into(), name.into()).is_err() || runtime.begin().is_err() {
        println!("ab_result variant={variant} setup=false"); return false;
    }
    let started = Instant::now();
    let mut fed = 0;
    let mut observations = Vec::new();
    let mut next = Duration::from_secs(1);
    let mut failed = false;
    let mut last = started;
    let mut max_gap = Duration::ZERO;
    let mut max_batch = 0;
    while started.elapsed() < Duration::from_secs(60) {
        let now = Instant::now(); max_gap = max_gap.max(now.duration_since(last)); last = now;
        let due = (started.elapsed().as_secs_f64() * 16000.0) as usize;
        let batch = due - fed; max_batch = max_batch.max(batch);
        if batch > 0 { if runtime.send(vec![0; batch]).is_err() { failed = true; break; } fed = due; }
        if started.elapsed() >= next {
            let state = runtime.snapshot();
            observations.push((started.elapsed().as_millis(), fed, state.submitted_samples, state.queued_samples));
            if state.phase == AudioPhase::Failed { failed = true; break; }
            next += Duration::from_secs(1);
        }
        thread::sleep(Duration::from_millis(15));
    }
    let stream_ms = started.elapsed().as_millis();
    let state = runtime.snapshot();
    let ending = Instant::now();
    let ended = if failed { runtime.interrupt() } else { runtime.finish() };
    let cleanup_ms = ending.elapsed().as_millis();
    let render_passed = !failed && ended.as_ref().is_ok_and(|s| s.phase == AudioPhase::Ready && s.submitted_samples == fed as u64);
    let consumer_passed = consumer.map_or(true, Consumer::finish);
    let passed = render_passed && consumer_passed;
    for (ms, fed, submitted, queued) in observations { println!("ab_flow variant={variant} elapsed_ms={ms} fed={fed} submitted={submitted} queued={queued}"); }
    println!("ab_result variant={variant} passed={passed} stream_ms={stream_ms} fed={fed} submitted={} queued={} phase={:?} producer_max_gap_us={} producer_max_batch={max_batch} cleanup_ok={} cleanup_ms={cleanup_ms}",state.submitted_samples,state.queued_samples,state.phase,max_gap.as_micros(),ended.is_ok());
    drop(runtime);
    println!("ab_drop variant={variant} completed=true");
    passed
}
fn main() {
    if std::env::args().any(|a| a == "--inspect-capture") { capture_context(); return; }
    use windows::Win32::System::Threading::*;
    // Same per-process policy as both production packages; no system policy change.
    let policy = PROCESS_POWER_THROTTLING_STATE { Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION, ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED | PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION, StateMask: 0 };
    unsafe { SetProcessInformation(GetCurrentProcess(), ProcessPowerThrottling, &policy as *const _ as _, std::mem::size_of_val(&policy) as u32).expect("match production process policy"); }
    let id = std::env::var("SAYALL_TEST_CABLE_ENDPOINT_ID").expect("saved endpoint required");
    let name = std::env::var("SAYALL_TEST_CABLE_ENDPOINT_NAME").expect("saved endpoint name required");
    let old = run(0, &id, &name);
    thread::sleep(Duration::from_secs(1));
    let new = run(1, &id, &name);
    println!("ab_complete old_passed={old} new_passed={new}");
    if !old || !new { std::process::exit(1); }
}

fn process_category(pid: u32) -> &'static str {
    use windows::{core::PWSTR, Win32::{Foundation::CloseHandle, System::Threading::*}};
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return "unknown"; };
        let mut path = [0u16; 1024]; let mut size = path.len() as u32;
        let result = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(path.as_mut_ptr()), &mut size);
        let _ = CloseHandle(handle);
        if result.is_err() { return "unknown"; }
        let text = String::from_utf16_lossy(&path[..size as usize]);
        let base = text.rsplit('\\').next().unwrap_or("").to_ascii_lowercase();
        if base.contains("wetype") { "wetype" } else { "other" }
    }
}
fn capture_context() {
    use wasapi::{DeviceEnumerator, Direction, Role, SessionState};
    if wasapi::initialize_mta().ok().is_err() { println!("capture_context error=com_initialize"); return; }
    struct Apartment; impl Drop for Apartment { fn drop(&mut self) { wasapi::deinitialize(); } }
    let _apartment = Apartment;
    let Ok(enumerator) = DeviceEnumerator::new() else { println!("capture_context error=enumerator"); return; };
    let classify = |name: &str| if name.to_ascii_lowercase().starts_with("cable output") { "cable_output" } else if name.to_ascii_lowercase().contains("bluetooth") || name.contains("耳机") { "headset" } else { "other" };
    for role in [Role::Console, Role::Multimedia, Role::Communications] {
        match enumerator.get_default_device_for_role(&Direction::Capture, &role).and_then(|d| d.get_friendlyname()) {
            Ok(name) => println!("capture_default role={role} category={} expected_cable_output={}", classify(&name), classify(&name) == "cable_output"),
            Err(_) => println!("capture_default role={role} query=failed"),
        }
    }
    let Ok(devices) = enumerator.get_device_collection(&Direction::Capture) else { println!("capture_context error=devices"); return; };
    for index in 0..devices.get_nbr_devices().unwrap_or(0) {
        let Ok(device) = devices.get_device_at_index(index) else { continue; };
        let Ok(name) = device.get_friendlyname() else { continue; };
        let category = classify(&name);
        let mut active = 0; let mut wetype = 0; let mut wetype_inactive = 0; let mut unknown = 0;
        let sessions = device.get_iaudiosessionmanager().and_then(|m| m.get_audiosessionenumerator());
        let observed = sessions.is_ok();
        if let Ok(sessions) = sessions {
            for i in 0..sessions.get_count().unwrap_or(0) {
                let Ok(session) = sessions.get_session(i) else { continue; };
                let running = session.get_state().is_ok_and(|s| s == SessionState::Active);
                let owner = session.get_process_id().map_or("unknown", process_category);
                if running { active += 1; if owner == "wetype" { wetype += 1; } if owner == "unknown" { unknown += 1; } }
                else if owner == "wetype" { wetype_inactive += 1; }
            }
        }
        println!("capture_sessions category={category} query_ok={observed} active={active} wetype_active={wetype} wetype_inactive={wetype_inactive} active_owner_unknown={unknown}");
    }
}

// Experiment-only explicit CABLE Output consumer. Public adapter metadata is
// corroboration for this unique standard cable, not a general device identity.
struct Consumer {
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<bool>>,
}
impl Consumer {
    fn finish(mut self) -> bool {
        self.stop.store(true, Ordering::SeqCst);
        self.thread.take().is_some_and(|thread| thread.join().unwrap_or(false))
    }
    fn start(render_id: &str, render_name: &str) -> Result<Self, &'static str> {
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let signal = stop.clone();
        let id = render_id.to_owned(); let name = render_name.to_owned();
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let thread = std::thread::spawn(move || {
            let result = consumer_loop(&id, &name, &signal, &tx);
            if let Err(stage) = result { let _ = tx.try_send(Err(stage)); println!("capture_consumer failed_stage={stage}"); }
            result.is_ok()
        });
        let mut owner = Self { stop, thread: Some(thread) };
        match rx.recv_timeout(Duration::from_secs(10)) {
            Ok(Ok(())) => Ok(owner),
            Ok(Err(stage)) => { owner.stop.store(true, Ordering::SeqCst); Err(stage) },
            Err(_) => { owner.stop.store(true, Ordering::SeqCst); Err("ready_timeout") },
        }
    }
}
impl Drop for Consumer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() { println!("capture_consumer joined={}", thread.join().is_ok()); }
    }
}
fn consumer_loop(render_id: &str, render_name: &str, stop: &std::sync::atomic::AtomicBool,
    ready: &std::sync::mpsc::SyncSender<Result<(), &'static str>>) -> Result<(), &'static str> {
    use windows::{core::HSTRING, Win32::{Media::Audio::*, System::Com::*}};
    wasapi::initialize_mta().ok().map_err(|_| "com")?;
    struct Apartment; impl Drop for Apartment { fn drop(&mut self) { wasapi::deinitialize(); } }
    let _apartment = Apartment;
    let enumeration = wasapi::DeviceEnumerator::new().map_err(|_| "enumerator")?;
    let render = enumeration.get_device(render_id).map_err(|_| "saved_render_missing")?;
    if render.get_friendlyname().map_err(|_| "render_name")? != render_name || render.get_description().map_err(|_| "render_description")? != "CABLE Input" { return Err("render_not_exact_standard_cable"); }
    let adapter = render.get_interface_friendlyname().map_err(|_| "render_adapter")?;
    if !adapter.to_ascii_lowercase().contains("vb-audio") { return Err("adapter_not_vb_audio"); }
    let devices = enumeration.get_device_collection(&wasapi::Direction::Capture).map_err(|_| "capture_enumeration")?;
    let mut candidates = Vec::new();
    for index in 0..devices.get_nbr_devices().map_err(|_| "capture_count")? {
        let d = devices.get_device_at_index(index).map_err(|_| "capture_device")?;
        if d.get_description().map_err(|_| "capture_description")? == "CABLE Output" {
            if d.get_interface_friendlyname().map_err(|_| "capture_adapter")? != adapter { return Err("adapter_mismatch"); }
            candidates.push((d.get_id().map_err(|_| "capture_id")?, d.get_friendlyname().map_err(|_| "capture_name")?));
        }
    }
    if candidates.len() != 1 { return Err("capture_not_unique"); }
    let (id, name) = &candidates[0];
    let check = enumeration.get_device(id).map_err(|_| "capture_reopen")?;
    if check.get_friendlyname().map_err(|_| "capture_reopen_name")? != *name || check.get_interface_friendlyname().map_err(|_| "capture_reopen_adapter")? != adapter { return Err("capture_changed"); }
    unsafe {
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|_| "native_enumerator")?;
        let device = enumerator.GetDevice(&HSTRING::from(id)).map_err(|_| "native_capture")?;
        let native_render = enumerator.GetDevice(&HSTRING::from(render_id)).map_err(|_| "native_render")?;
        let render_container = container_id(&native_render)?;
        let capture_container = container_id(&device)?;
        if render_container == windows::core::GUID::zeroed() || render_container != capture_container { return Err("container_mismatch_or_empty"); }
        println!("capture_consumer binding=public_container_equal_nonzero exact_names=true unique_capture=true");
        let client: IAudioClient = device.Activate(CLSCTX_ALL, None).map_err(|_| "activate")?;
        let format = client.GetMixFormat().map_err(|_| "mix_format")?;
        if format.is_null() { return Err("null_mix_format"); }
        let rate = (*format).nSamplesPerSec; let channels = (*format).nChannels; let bits = (*format).wBitsPerSample;
        let init = client.Initialize(AUDCLNT_SHAREMODE_SHARED, 0, 1_000_000, 0, format, None);
        CoTaskMemFree(Some(format.cast()));
        init.map_err(|_| "initialize")?;
        let capacity = client.GetBufferSize().map_err(|_| "capacity")?;
        let capture: IAudioCaptureClient = client.GetService().map_err(|_| "capture_service")?;
        client.Start().map_err(|_| "start")?;
        struct Stop<'a>(&'a IAudioClient, bool); impl Drop for Stop<'_> { fn drop(&mut self) { if self.1 { println!("capture_consumer abort_stop_ok={}", unsafe { self.0.Stop().is_ok() }); } } }
        let mut stop_stream = Stop(&client, true);
        println!("capture_consumer started=true unique_standard_cable=true adapter_match=true sample_rate={rate} channels={channels} bits={bits} capacity_frames={capacity} audio_bytes_saved=0");
        ready.send(Ok(())).map_err(|_| "ready_receiver_closed")?;
        let start = Instant::now(); let mut frames = 0u64; let mut packets = 0u64; let mut discontinuities = 0u64; let mut sent_ready = false;
        let mut observations = Vec::new(); let mut next = Duration::from_secs(10);
        while !stop.load(Ordering::SeqCst) && start.elapsed() < Duration::from_secs(75) {
            while !stop.load(Ordering::SeqCst) && start.elapsed() < Duration::from_secs(75) {
                if capture.GetNextPacketSize().map_err(|_| "packet_size")? == 0 { break; }
                let mut data = std::ptr::null_mut(); let mut count = 0; let mut flags = 0;
                capture.GetBuffer(&mut data, &mut count, &mut flags, None, None).map_err(|_| "get_buffer")?;
                // Deliberately never dereference data: release/discard only, including SILENT.
                capture.ReleaseBuffer(count).map_err(|_| "release_buffer")?;
                frames += count as u64; packets += 1;
                if flags & AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY.0 as u32 != 0 { discontinuities += 1; }
                if count > 0 { sent_ready = true; }
            }
            if start.elapsed() >= next { observations.push((start.elapsed().as_millis(), frames)); next += Duration::from_secs(10); }
            std::thread::sleep(Duration::from_millis(5));
        }
        for (elapsed_ms, frames) in observations { println!("capture_consumer_flow elapsed_ms={elapsed_ms} frames={frames} audio_duration_ms={}",frames * 1000 / rate as u64); }
        println!("capture_consumer completed=true elapsed_ms={} frames={frames} audio_duration_ms={} packets={packets} discontinuities={discontinuities} ready={sent_ready}",start.elapsed().as_millis(),frames * 1000 / rate as u64);
        if !sent_ready { return Err("no_capture_frames"); }
        client.Stop().map_err(|_| "stop")?;
        stop_stream.1 = false;
        println!("capture_consumer stop_ok=true");
    }
    Ok(())
}

fn container_id(device: &windows::Win32::Media::Audio::IMMDevice) -> Result<windows::core::GUID, &'static str> {
    use windows::Win32::{Devices::FunctionDiscovery::PKEY_Device_ContainerId, System::Com::{STGM_READ, StructuredStorage::{PropVariantToGUID, PropVariantClear}}};
    unsafe {
        let store = device.OpenPropertyStore(STGM_READ).map_err(|_| "container_store")?;
        let mut value = store.GetValue(&PKEY_Device_ContainerId).map_err(|_| "container_value")?;
        let id = PropVariantToGUID(&value).map_err(|_| "container_guid");
        PropVariantClear(&mut value).map_err(|_| "container_clear")?;
        id
    }
}

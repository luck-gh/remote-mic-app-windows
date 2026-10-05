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
    let passed = !failed && ended.as_ref().is_ok_and(|s| s.phase == AudioPhase::Ready && s.submitted_samples == fed as u64);
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

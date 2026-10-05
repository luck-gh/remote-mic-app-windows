use std::time::{Duration,Instant,SystemTime,UNIX_EPOCH};
fn main() {
    let stop = std::env::current_exe().expect("executable path").with_file_name("capture-route-watch.stop");
    if stop.exists() { println!("capture_watch blocked=existing_stop_marker"); return; }
    let started = Instant::now(); let mut previous = Vec::new(); let mut consecutive_errors = 0;
    let once = std::env::args().any(|a| a == "--once");
    println!("capture_watch ready=true read_only=true audio_read=false limit_seconds=1800");
    while started.elapsed() < Duration::from_secs(1800) && !stop.exists() {
        let before=Instant::now(); let rows=capture_context();
        let failed = rows.iter().any(|r| r.contains("error=") || r.contains("query=failed") || r.contains("query_ok=false"));
        consecutive_errors = if failed { consecutive_errors + 1 } else { 0 };
        if rows != previous {
            let unix_ms=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
            println!("capture_watch unix_ms={unix_ms} query_elapsed_ms={} {}",before.elapsed().as_millis(),rows.join(" | "));
            previous=rows;
        }
        if once || consecutive_errors >= 3 { break; }
        std::thread::sleep(Duration::from_secs(1));
    }
    println!("capture_watch stopped=true elapsed_ms={}",started.elapsed().as_millis());
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
fn capture_context() -> Vec<String> {
    let mut rows = Vec::new();
    use wasapi::{DeviceEnumerator, Direction, Role, SessionState};
    if wasapi::initialize_mta().ok().is_err() { rows.push(format!("capture_context error=com_initialize")); return rows; }
    struct Apartment; impl Drop for Apartment { fn drop(&mut self) { wasapi::deinitialize(); } }
    let _apartment = Apartment;
    let Ok(enumerator) = DeviceEnumerator::new() else { rows.push(format!("capture_context error=enumerator")); return rows; };
    let classify = |name: &str| if name.to_ascii_lowercase().starts_with("cable output") { "cable_output" } else if name.to_ascii_lowercase().contains("bluetooth") || name.contains("耳机") { "headset" } else { "other" };
    for role in [Role::Console, Role::Multimedia, Role::Communications] {
        match enumerator.get_default_device_for_role(&Direction::Capture, &role).and_then(|d| d.get_friendlyname()) {
            Ok(name) => rows.push(format!("capture_default role={role} category={} expected_cable_output={}", classify(&name), classify(&name) == "cable_output")),
            Err(_) => rows.push(format!("capture_default role={role} query=failed")),
        }
    }
    let Ok(devices) = enumerator.get_device_collection(&Direction::Capture) else { rows.push(format!("capture_context error=devices")); return rows; };
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
                let Ok(state) = session.get_state() else { rows.push(format!("capture_session error=state_query")); continue; };
                let running = state == SessionState::Active;
                let owner = session.get_process_id().map_or("unknown", process_category);
                if running { active += 1; if owner == "wetype" { wetype += 1; } if owner == "unknown" { unknown += 1; } }
                else if owner == "wetype" { wetype_inactive += 1; }
            }
        }
        rows.push(format!("capture_sessions category={category} query_ok={observed} active={active} wetype_active={wetype} wetype_inactive={wetype_inactive} active_owner_unknown={unknown}"));
    }
    rows
}

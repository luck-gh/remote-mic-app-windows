//! Opt-in, silent RTWQ mechanism experiment; excluded from product binaries.
//! Microsoft Low Latency Audio / RTWQ waiting-item pattern. No copied sample code.
//! Keeps the original format, device period, 30 ms prebuffer and 2 s PCM limit.
use super::*;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use windows::core::{implement, w, AgileReference, Ref, Result as WinResult, HSTRING};
use windows::Win32::Foundation::{E_FAIL, E_POINTER, HANDLE};
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
use windows::Win32::System::Threading::*;

struct State {
    client: Option<AgileReference<IAudioClient>>,
    event: OwnedHandle,
    capacity: u32,
    accepting_callbacks: bool,
    started: bool,
    draining: bool,
    drained: bool,
    pending_key: Option<u64>,
    queue: VecDeque<i16>,
    submitted: u64,
    failure: Option<i32>,
    calls: u64,
    last_call: Option<Instant>,
    max_gap: Duration,
    gap_excess: Duration,
    gap_over_50ms: u64,
    period: Duration,
    begin_at: Instant,
    first_write_ms: Option<u128>,
    first_write_frames: usize,
    last_padding: u32,
}

#[implement(IRtwqAsyncCallback)]
struct RenderCallback {
    state: Arc<Mutex<State>>,
    queue_id: u32,
}

impl IRtwqAsyncCallback_Impl for RenderCallback_Impl {
    fn GetParameters(&self, flags: *mut u32, queue: *mut u32) -> WinResult<()> {
        if flags.is_null() || queue.is_null() {
            return Err(E_POINTER.into());
        }
        unsafe {
            *flags = 0;
            *queue = self.queue_id;
        }
        Ok(())
    }

    fn Invoke(&self, result: Ref<IRtwqAsyncResult>) -> WinResult<()> {
        // Balance COM on the callback thread. No assumption that serial FIFO
        // callbacks retain thread identity, and no unsafe Send/Sync wrappers.
        wasapi::initialize_mta().ok()?;
        let _apartment = WasapiApartment;
        let mut state = lock(&self.state);
        if !state.accepting_callbacks {
            return Ok(());
        }
        state.pending_key = None;
        let action = (|| -> WinResult<()> {
            let now = Instant::now();
            if state.started {
                if let Some(last) = state.last_call {
                    let gap = now.duration_since(last);
                    state.max_gap = state.max_gap.max(gap);
                    if gap > state.period {
                        let period = state.period;
                        state.gap_excess += gap - period;
                    }
                    if gap > Duration::from_millis(50) {
                        state.gap_over_50ms += 1;
                    }
                }
            }
            state.last_call = Some(now);
            state.calls += 1;
            let client = state.client.as_ref().ok_or(E_FAIL)?.resolve()?;
            if state.started || state.draining || state.queue.len() >= PREBUFFER_SAMPLES {
                let padding = unsafe { client.GetCurrentPadding()? };
                state.last_padding = padding;
                let frames = state
                    .capacity
                    .saturating_sub(padding)
                    .min(state.queue.len() as u32);
                if frames > 0 {
                    // GetService, GetBuffer, ReleaseBuffer and interface Release
                    // all occur in this invocation on this exact thread.
                    let render: IAudioRenderClient = unsafe { client.GetService()? };
                    let data = unsafe { render.GetBuffer(frames)? };
                    for (index, sample) in state.queue.drain(..frames as usize).enumerate() {
                        let bytes = sample.to_le_bytes();
                        unsafe {
                            std::ptr::copy_nonoverlapping(bytes.as_ptr(), data.add(index * 2), 2);
                        }
                    }
                    unsafe {
                        render.ReleaseBuffer(frames, 0)?;
                    }
                    drop(render);
                    state.submitted += u64::from(frames);
                    if !state.started {
                        unsafe {
                            client.Start()?;
                        }
                        state.started = true;
                        state.first_write_ms = Some(state.begin_at.elapsed().as_millis());
                        state.first_write_frames = frames as usize;
                        state.last_call = Some(Instant::now());
                    }
                }
                if state.draining
                    && state.queue.is_empty()
                    && unsafe { client.GetCurrentPadding()? } == 0
                {
                    state.drained = true;
                }
            }
            if !state.drained {
                let result = result.as_ref().ok_or(E_POINTER)?;
                let mut key = 0;
                unsafe {
                    RtwqPutWaitingWorkItem(
                        HANDLE(state.event.as_raw_handle()),
                        0,
                        result,
                        Some(&mut key),
                    )?;
                }
                state.pending_key = Some(key);
            }
            Ok(())
        })();
        if let Err(error) = action {
            state.failure = Some(error.code().0);
        }
        Ok(())
    }
}

#[implement(IRtwqAsyncCallback)]
struct Barrier {
    queue_id: u32,
    done: Sender<()>,
}
impl IRtwqAsyncCallback_Impl for Barrier_Impl {
    fn GetParameters(&self, flags: *mut u32, queue: *mut u32) -> WinResult<()> {
        if flags.is_null() || queue.is_null() {
            return Err(E_POINTER.into());
        }
        unsafe {
            *flags = 0;
            *queue = self.queue_id;
        }
        Ok(())
    }
    fn Invoke(&self, _: Ref<IRtwqAsyncResult>) -> WinResult<()> {
        let _ = self.done.send(());
        Ok(())
    }
}

struct WorkQueues {
    shared: Option<u32>,
    serial: Option<u32>,
}
impl Drop for WorkQueues {
    fn drop(&mut self) {
        unsafe {
            if let Some(queue) = self.serial {
                let result = RtwqUnlockWorkQueue(queue);
                println!("rtwq_cleanup serial_unlock={}", result.is_ok());
            }
            if let Some(queue) = self.shared {
                let result = RtwqUnlockWorkQueue(queue);
                println!("rtwq_cleanup shared_unlock={}", result.is_ok());
            }
            println!("rtwq_cleanup shutdown={}", RtwqShutdown().is_ok());
        }
    }
}

#[test]
#[ignore = "explicit saved endpoint; 60s silent RTWQ refill experiment; no product changes"]
fn saved_endpoint_rtwq_steady_silence() {
    crate::power::disable_background_power_throttling().unwrap();
    wasapi::initialize_mta().ok().unwrap();
    let _apartment = WasapiApartment;
    let id = std::env::var("SAYALL_TEST_CABLE_ENDPOINT_ID").unwrap();
    let name = std::env::var("SAYALL_TEST_CABLE_ENDPOINT_NAME").unwrap();
    assert!(list_endpoints()
        .unwrap()
        .iter()
        .any(|e| e.id == id && e.name == name));
    assert!(
        !unsafe { endpoint_volume(&id).unwrap().GetMute().unwrap().as_bool() },
        "refuse muted endpoint"
    );
    let enumerator: IMMDeviceEnumerator =
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).unwrap() };
    let device = unsafe { enumerator.GetDevice(&HSTRING::from(&id)).unwrap() };
    let client: IAudioClient = unsafe { device.Activate(CLSCTX_ALL, None).unwrap() };
    let format = WaveFormat::new(
        16,
        16,
        &SampleType::Int,
        SOURCE_SAMPLE_RATE,
        SOURCE_CHANNELS,
        None,
    );
    let mut period = 0;
    unsafe {
        client.GetDevicePeriod(Some(&mut period), None).unwrap();
        client
            .Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_EVENTCALLBACK
                    | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
                    | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
                period,
                0,
                format.as_waveformatex_ref(),
                None,
            )
            .unwrap();
    }
    let capacity = unsafe { client.GetBufferSize().unwrap() };
    let event =
        unsafe { OwnedHandle::from_raw_handle(CreateEventW(None, false, false, None).unwrap().0) };
    unsafe {
        client
            .SetEventHandle(HANDLE(event.as_raw_handle()))
            .unwrap();
    }
    let reference = AgileReference::new(&client).expect("marshal audio client without unsafe Send");
    unsafe {
        RtwqStartup().unwrap();
    }
    let mut queues = WorkQueues {
        shared: None,
        serial: None,
    };
    let (mut task, mut shared) = (0, 0);
    unsafe {
        RtwqLockSharedWorkQueue(w!("Audio"), 0, &mut task, &mut shared).unwrap();
    }
    queues.shared = Some(shared);
    let serial = unsafe { RtwqAllocateSerialWorkQueue(shared).unwrap() };
    queues.serial = Some(serial);
    let state = Arc::new(Mutex::new(State {
        client: Some(reference),
        event,
        capacity,
        accepting_callbacks: true,
        started: false,
        draining: false,
        drained: false,
        pending_key: None,
        queue: VecDeque::new(),
        submitted: 0,
        failure: None,
        calls: 0,
        last_call: None,
        max_gap: Duration::ZERO,
        gap_excess: Duration::ZERO,
        gap_over_50ms: 0,
        period: Duration::from_nanos(period as u64 * 100),
        begin_at: Instant::now(),
        first_write_ms: None,
        first_write_frames: 0,
        last_padding: 0,
    }));
    let callback: IRtwqAsyncCallback = RenderCallback {
        state: state.clone(),
        queue_id: serial,
    }
    .into();
    let result = unsafe { RtwqCreateAsyncResult(None, &callback, None).unwrap() };
    {
        let mut state = lock(&state);
        let mut key = 0;
        unsafe {
            RtwqPutWaitingWorkItem(
                HANDLE(state.event.as_raw_handle()),
                0,
                &result,
                Some(&mut key),
            )
            .unwrap();
        }
        state.pending_key = Some(key);
    }
    println!("rtwq_started capacity={capacity} period_hns={period} source_rate=16000 prebuffer=480 pcm_limit=32000");
    let started = Instant::now();
    let mut fed = 0usize;
    let mut next_snapshot = Duration::from_secs(1);
    let mut observations = Vec::new();
    let mut producer_last = Instant::now();
    let mut producer_gap_max = Duration::ZERO;
    let mut producer_batch_max = 0;
    while started.elapsed() < Duration::from_secs(60) {
        let now = Instant::now();
        producer_gap_max = producer_gap_max.max(now.duration_since(producer_last));
        producer_last = now;
        let due = (started.elapsed().as_secs_f64() * SOURCE_SAMPLE_RATE as f64) as usize;
        {
            let mut state = lock(&state);
            if state.failure.is_some() {
                break;
            }
            let batch = due - fed;
            if state.queue.len() + batch > MAX_QUEUE_SAMPLES {
                state.failure = Some(-1);
                break;
            }
            producer_batch_max = producer_batch_max.max(batch);
            state.queue.extend(std::iter::repeat_n(0, batch));
            fed = due;
            if !state.started && state.queue.len() >= PREBUFFER_SAMPLES {
                unsafe {
                    SetEvent(HANDLE(state.event.as_raw_handle())).unwrap();
                }
            }
            if started.elapsed() >= next_snapshot {
                observations.push((
                    started.elapsed().as_millis(),
                    fed,
                    state.submitted,
                    state.queue.len(),
                ));
                next_snapshot += Duration::from_secs(1);
            }
        }
        thread::sleep(Duration::from_millis(15));
    }
    let drain_started = Instant::now();
    {
        let mut state = lock(&state);
        state.draining = true;
        unsafe {
            SetEvent(HANDLE(state.event.as_raw_handle())).unwrap();
        }
    }
    while drain_started.elapsed() < DRAIN_TIMEOUT {
        {
            let state = lock(&state);
            if state.drained || state.failure.is_some() {
                break;
            }
        }
        thread::sleep(POLL_INTERVAL);
    }
    let drain_ms = drain_started.elapsed().as_millis();
    // Cancel alone does not guarantee Invoke cannot arrive. Close the gate under
    // the same state lock, then FIFO barrier. Late callbacks retain Arc<State>
    // and return before touching the client/event or rescheduling themselves.
    let key = {
        let mut state = lock(&state);
        state.accepting_callbacks = false;
        state.pending_key.take()
    };
    if let Some(key) = key {
        let cancelled = unsafe { RtwqCancelWorkItem(key) };
        println!(
            "rtwq_cleanup cancel_ok={} code={}",
            cancelled.is_ok(),
            cancelled.err().map_or(0, |e| e.code().0)
        );
    }
    let (done, finished) = mpsc::channel();
    let barrier: IRtwqAsyncCallback = Barrier {
        queue_id: serial,
        done,
    }
    .into();
    let barrier_result = unsafe { RtwqCreateAsyncResult(None, &barrier, None).unwrap() };
    unsafe {
        RtwqPutWorkItem(serial, 0, &barrier_result).unwrap();
    }
    let barrier_passed = finished.recv_timeout(Duration::from_secs(5)).is_ok();
    let mut state = lock(&state);
    let stop = unsafe { client.Stop() };
    state.client.take();
    for (elapsed, accepted, submitted, queued) in observations {
        println!("rtwq_flow elapsed_ms={elapsed} accepted={accepted} submitted={submitted} queued={queued}");
    }
    println!("rtwq_terminal elapsed_ms={} fed={fed} submitted={} queued={} failure={:?} calls={} max_gap_us={} gap_excess_us={} over50ms={} first_write_ms={:?} first_write_frames={} drain_ms={drain_ms} drained={} producer_max_gap_us={} producer_max_batch={producer_batch_max} barrier={barrier_passed} stop={}", started.elapsed().as_millis(),state.submitted,state.queue.len(),state.failure,state.calls,state.max_gap.as_micros(),state.gap_excess.as_micros(),state.gap_over_50ms,state.first_write_ms,state.first_write_frames,state.drained,producer_gap_max.as_micros(),stop.is_ok());
    let passed = state.failure.is_none()
        && state.drained
        && state.submitted == fed as u64
        && barrier_passed
        && stop.is_ok();
    drop(state);
    drop(result);
    drop(callback);
    drop(barrier_result);
    drop(barrier);
    drop(client);
    drop(queues);
    assert!(passed, "RTWQ mechanism failed; terminal counts retained");
}

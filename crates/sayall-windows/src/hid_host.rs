//! Device-bound RC003 input enhancement. ATVV remains independent.
#[cfg(windows)]
#[path = "hid_host_windows.rs"]
mod windows_impl;
#[cfg(windows)]
pub(crate) use windows_impl::start;
#[cfg(windows)]
pub use windows_impl::{cancel_pending_start, current_status, request_start, restore_on_start};
pub(crate) fn packaged() -> bool {
    option_env!("SAYALL_HID_HOST_HELPER_SHA256").is_some()
}
#[cfg(not(windows))]
pub fn request_start() -> Result<String, String> {
    Err("三键增强需要 Windows。".into())
}
#[cfg(not(windows))]
pub fn current_status() -> String {
    "需要 Windows".into()
}
#[cfg(not(windows))]
pub fn restore_on_start() {}
#[cfg(not(windows))]
pub fn cancel_pending_start() {}
use crate::raw_input::{ButtonEdge, RemoteButton};

pub const PROTOCOL: u32 = 1;
pub(crate) const BUTTONS: [RemoteButton; 5] = [
    RemoteButton::Back,
    RemoteButton::VolumeUp,
    RemoteButton::VolumeDown,
    RemoteButton::Tv,
    RemoteButton::Home,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Report {
    pub buttons: u8,
    pub all_released: bool,
}

/// WDF output after the Helper has verified the selected HID descriptor.
/// 121 is the collection maximum, not a guessed physical completion length.
pub fn decode_report(bytes: &[u8]) -> Result<Report, &'static str> {
    if !(7..=121).contains(&bytes.len()) || bytes[0] != 1 || bytes[7..].iter().any(|b| *b != 0) {
        return Err("host_report_shape");
    }
    let mut buttons = 0;
    let mut all_released = true;
    for slot in bytes[1..7].chunks_exact(2) {
        let usage = u16::from_le_bytes([slot[0], slot[1]]);
        if usage > 254 || (1..=3).contains(&usage) {
            return Err("host_report_usage");
        }
        all_released &= usage == 0;
        buttons |= match usage {
            0xf1 => 1,
            0x80 => 2,
            0x81 => 4,
            0x35 => 8,
            0x4a => 16,
            _ => 0,
        };
    }
    Ok(Report {
        buttons,
        all_released,
    })
}

/// A new generation waits for a physically observed empty report. Repeated
/// reports are not DOWN repeats. Cancellation releases delivered mapping edges
/// and cannot turn a held key into a new DOWN when the transport returns.
#[derive(Default)]
pub struct Edges {
    ready: bool,
    delivered: u8,
    sequence: Option<u64>,
}

pub(crate) fn cancel_mapping(
    mapping: &crate::button_mapping::ButtonMappingRuntime,
    edges: &mut Edges,
) {
    mapping.set_input_enhancement(false);
    let released = edges.cancel();
    crate::gatt_note(format!(
        "hid_host phase=cancel_release count={}",
        released.len()
    ));
    for edge in released {
        let _ = mapping
            .sender()
            .send(crate::button_mapping::EngineMessage::DriverEdge(edge));
    }
}

impl Edges {
    pub fn ready(&self) -> bool {
        self.ready
    }
    pub fn accept(
        &mut self,
        sequence: u64,
        report: Report,
    ) -> Result<Vec<ButtonEdge>, &'static str> {
        self.accept_verified(sequence, report, false)
    }
    /// The exact Helper may own the first DOWN after a continuously observed
    /// released reconnect. Caller validates its ready/suppressed wire contract.
    pub(crate) fn accept_verified(
        &mut self,
        sequence: u64,
        report: Report,
        helper_owned: bool,
    ) -> Result<Vec<ButtonEdge>, &'static str> {
        if report.buttons & !31 != 0 || (report.all_released && report.buttons != 0) {
            return Err("host_mask_invalid");
        }
        if self
            .sequence
            .is_some_and(|last| sequence != last.wrapping_add(1))
        {
            return Err("host_sequence_gap");
        }
        self.sequence = Some(sequence);
        if helper_owned {
            self.ready = true;
        }
        if !self.ready {
            self.ready = report.all_released;
            return Ok(Vec::new());
        }
        let mut result = Vec::new();
        for pressed in [false, true] {
            for (index, button) in BUTTONS.into_iter().enumerate() {
                let bit = 1 << index;
                if self.delivered & bit != report.buttons & bit
                    && (report.buttons & bit != 0) == pressed
                {
                    result.push(ButtonEdge {
                        button,
                        is_pressed: pressed,
                    });
                }
            }
        }
        self.delivered = report.buttons;
        Ok(result)
    }
    pub fn cancel(&mut self) -> Vec<ButtonEdge> {
        let released = BUTTONS
            .into_iter()
            .enumerate()
            .filter_map(|(i, button)| {
                (self.delivered & (1 << i) != 0).then_some(ButtonEdge {
                    button,
                    is_pressed: false,
                })
            })
            .collect();
        *self = Self::default();
        released
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn report(usages: [u16; 3]) -> Report {
        let mut bytes = vec![1];
        for usage in usages {
            bytes.extend(usage.to_le_bytes());
        }
        decode_report(&bytes).unwrap()
    }
    #[test]
    fn exact_host_contract_slots_and_non_target_keys() {
        assert_eq!(
            report([0xf1, 0x80, 0x81]),
            Report {
                buttons: 7,
                all_released: false
            }
        );
        assert_eq!(
            report([0x80, 0x80, 0]),
            Report {
                buttons: 2,
                all_released: false
            }
        );
        assert_eq!(
            report([0x3e, 0x28, 0x4f]),
            Report {
                buttons: 0,
                all_released: false
            }
        );
        assert!(report([0, 0, 0]).all_released);
        for data in [
            vec![],
            vec![0; 9],
            vec![1, 0, 0, 0, 1, 0, 0, 0, 0],
            vec![1, 0, 0, 1, 0, 0, 0, 0, 0],
        ] {
            assert!(decode_report(&data).is_err());
        }
    }
    #[test]
    fn fast_and_combined_edges_are_paired_without_duplicates() {
        let mut state = Edges::default();
        assert!(state.accept(0, report([0, 0, 0])).unwrap().is_empty());
        let mut sequence = 1;
        for _ in 0..20 {
            let down = state.accept(sequence, report([0xf1, 0x80, 0x81])).unwrap();
            sequence += 1;
            assert_eq!(down.len(), 3);
            assert!(down.iter().all(|e| e.is_pressed));
            assert!(state
                .accept(sequence, report([0xf1, 0x80, 0x81]))
                .unwrap()
                .is_empty());
            sequence += 1;
            let up = state.accept(sequence, report([0, 0, 0])).unwrap();
            sequence += 1;
            assert_eq!(up.len(), 3);
            assert!(up.iter().all(|e| !e.is_pressed));
        }
    }
    #[test]
    fn reconnect_cancel_and_held_start_wait_for_physical_release() {
        let mut state = Edges::default();
        assert!(state.accept(8, report([0xf1, 0, 0])).unwrap().is_empty());
        assert!(!state.ready());
        state.accept(9, report([0, 0, 0])).unwrap();
        assert_eq!(state.accept(10, report([0xf1, 0, 0])).unwrap().len(), 1);
        assert_eq!(state.cancel().len(), 1);
        assert!(state.cancel().is_empty());
        assert!(state.accept(0, report([0xf1, 0, 0])).unwrap().is_empty());
        assert!(!state.ready());
        state.accept(1, report([0, 0, 0])).unwrap();
        assert!(state.ready());
        assert_eq!(state.accept(2, report([0xf1, 0, 0])).unwrap().len(), 1);
        assert!(state.accept(4, report([0, 0, 0])).is_err());
        assert_eq!(state.cancel().len(), 1);
    }
}

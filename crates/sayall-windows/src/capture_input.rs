//! Capture routing for voice sessions and explicit one-shot default selection.
//! Both share the restricted setter and journal. Notifications never prove an actor.
use sayall_core::CaptureInputSettings;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureInputSnapshot {
    pub settings: CaptureInputSettings,
    pub phase: String,
    pub recovery_pending: bool,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultCaptureInputs {
    pub console: Option<crate::AudioEndpoint>,
    pub multimedia: Option<crate::AudioEndpoint>,
    pub communications: Option<crate::AudioEndpoint>,
}

fn manual_default(
    operation_id: u64,
    backend: &mut impl RouteBackend,
    target: String,
    name: String,
    check: impl Fn() -> Result<(), String>,
) -> Result<(), String> {
    check()?;
    if backend.active_name(&target)? != name {
        return Err("capture_target_changed".into());
    }
    let original = backend.roles()?;
    let mut names = [None, None, None];
    for (index, id) in original.iter().enumerate() {
        if let Some(id) = id {
            names[index] = Some(backend.active_name(id)?);
        }
    }
    let mut transaction = Some(Transaction {
        version: 1,
        generation: operation_id,
        target,
        target_name: name,
        original: original.clone(),
        original_names: names,
        expected: original,
        changed: [false; 3],
        transition: None,
    });
    let result = prepare_route(backend, &mut transaction, true, || check().is_err());
    match result {
        Ok(_) => Ok(()),
        Err(error) => {
            if let Some(mut tx) = transaction {
                match restore(backend, &mut tx) {
                    Ok(()) => {}
                    Err(rollback_error) if rollback_error == "external_change" => {
                        backend.persist(None)?
                    }
                    Err(rollback_error) => {
                        return Err(format!("manual_default_rollback_failed:{rollback_error}"))
                    }
                }
            }
            Err(error)
        }
    }
}

#[cfg(windows)]
#[path = "capture_input_windows.rs"]
mod windows;
#[cfg(windows)]
pub(crate) use windows::CaptureInputRuntime;

type Roles = [Option<String>; 3];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Transaction {
    version: u32,
    generation: u64,
    target: String,
    target_name: String,
    original: Roles,
    original_names: [Option<String>; 3],
    expected: Roles,
    // Persisted BEFORE each setter: success followed immediately by a crash is covered.
    changed: [bool; 3],
    transition: Option<Transition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Transition {
    role: usize,
    mask: u8,
    before: Roles,
    desired: Roles,
}

/// The only test seam is the actual side-effect boundary. Production uses MMDevice,
/// IPolicyConfig and a write-through journal; tests never change Windows defaults.
trait RouteBackend {
    fn roles(&mut self) -> Result<Roles, String>;
    fn active_name(&mut self, id: &str) -> Result<String, String>;
    fn set(&mut self, role: usize, id: &str) -> Result<(), String>;
    fn persist(&mut self, value: Option<&Transaction>) -> Result<(), String>;
    fn observe(&mut self, _stage: &str, _tx: &Transaction, _actual: &Roles) {}
}

fn check_expected(backend: &mut impl RouteBackend, tx: &Transaction) -> Result<(), String> {
    if backend.roles()? != tx.expected {
        return Err("external_change".into());
    }
    Ok(())
}

fn group_mask(role: usize) -> u8 {
    if role < 2 {
        0b011
    } else {
        0b100
    }
}

fn transition_matches(t: &Transition, actual: &Roles) -> bool {
    t.role < 3
        && t.mask == group_mask(t.role)
        && (0..3).all(|i| {
            actual[i] == t.before[i] || (t.mask & (1 << i) != 0 && actual[i] == t.desired[i])
        })
}

fn validate_recovery(tx: &Transaction, actual: &Roles) -> Result<(), String> {
    let matches = match &tx.transition {
        Some(t) => t.before == tx.expected && transition_matches(t, actual),
        None => *actual == tx.expected,
    };
    if matches {
        Ok(())
    } else {
        Err("recovery_external_change".into())
    }
}

fn confirm_actual(tx: &mut Transaction, actual: Roles) {
    tx.changed = std::array::from_fn(|i| actual[i] != tx.original[i]);
    tx.expected = actual;
    tx.transition = None;
}

/// Console/Multimedia may converge together. This envelope accepts only the
/// group's intended value, never infers an actor from a notification or timing.
fn set_group(
    backend: &mut impl RouteBackend,
    tx: &mut Transaction,
    role: usize,
    id: &str,
    check: &impl Fn() -> Result<(), String>,
) -> Result<(), String> {
    check()?;
    check_expected(backend, tx)?;
    let mask = group_mask(role);
    let mut desired = tx.expected.clone();
    for (i, value) in desired.iter_mut().enumerate() {
        if mask & (1 << i) != 0 {
            *value = Some(id.to_owned());
        }
    }
    let transition = Transition {
        role,
        mask,
        before: tx.expected.clone(),
        desired,
    };
    tx.transition = Some(transition.clone());
    backend.persist(Some(tx))?;
    check()?;
    check_expected(backend, tx)?;
    backend.observe("before_set", tx, &transition.before);
    check()?;
    let result = backend.set(role, id);
    let actual = backend.roles()?;
    backend.observe("after_set", tx, &actual);
    if !transition_matches(&transition, &actual) {
        return Err("external_change".into());
    }
    let confirmed = actual[role].as_deref() == Some(id);
    confirm_actual(tx, actual);
    backend.persist(Some(tx))?;
    result?;
    if !confirmed {
        return Err("switch_not_confirmed".into());
    }
    check()
}

fn apply(
    backend: &mut impl RouteBackend,
    tx: &mut Transaction,
    cancelled: impl Fn() -> bool,
) -> Result<(), String> {
    if tx
        .original
        .iter()
        .all(|id| id.as_deref() == Some(&tx.target))
    {
        if cancelled() {
            return Err("cancelled".into());
        }
        if backend.active_name(&tx.target)? != tx.target_name {
            return Err("target_changed".into());
        }
        check_expected(backend, tx)?;
        if cancelled() {
            return Err("cancelled".into());
        }
        backend.observe("already_default", tx, &tx.expected);
        return Ok(());
    }
    // A split normal-role baseline cannot be restored reliably with a setter
    // that may couple those roles. Reject before the first journal or write.
    if tx.original[0] != tx.original[1] {
        return Err("normal_roles_split".into());
    }
    if tx.original.iter().any(Option::is_none) {
        return Err("original_missing".into());
    }
    backend.observe("baseline", tx, &tx.original);
    backend.persist(Some(tx))?;
    let check = || {
        if cancelled() {
            Err("cancelled".into())
        } else {
            Ok(())
        }
    };
    for role in 0..3 {
        check()?;
        check_expected(backend, tx)?;
        if tx.expected[role].as_deref() == Some(&tx.target) {
            continue;
        }
        if backend.active_name(&tx.target)? != tx.target_name {
            return Err("target_changed".into());
        }
        let target = tx.target.clone();
        set_group(backend, tx, role, &target, &check)?;
    }
    check_expected(backend, tx)?;
    if cancelled() {
        return Err("cancelled".into());
    }
    Ok(())
}

fn restore(backend: &mut impl RouteBackend, tx: &mut Transaction) -> Result<(), String> {
    restore_checked(backend, tx, || Ok(()))
}

fn preserve(
    backend: &mut impl RouteBackend,
    tx: &Transaction,
    cancelled: impl Fn() -> bool,
) -> Result<(), String> {
    if cancelled() {
        return Err("cancelled".into());
    }
    check_expected(backend, tx)?;
    if cancelled() {
        return Err("cancelled".into());
    }
    backend.persist(None)?;
    if cancelled() {
        return Err("cancelled".into());
    }
    backend.observe("preserved", tx, &tx.expected);
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum RoutePreparation {
    AlreadyDefault,
    Temporary,
    Preserved,
}

fn prepare_route(
    backend: &mut impl RouteBackend,
    transaction: &mut Option<Transaction>,
    keep_selected: bool,
    cancelled: impl Fn() -> bool,
) -> Result<RoutePreparation, String> {
    let tx = transaction.as_mut().unwrap();
    let already_default = tx
        .original
        .iter()
        .all(|id| id.as_deref() == Some(&tx.target));
    let result = apply(backend, tx, &cancelled);
    if already_default {
        *transaction = None;
        return result.map(|_| RoutePreparation::AlreadyDefault);
    }
    result?;
    if keep_selected {
        preserve(backend, transaction.as_ref().unwrap(), cancelled)?;
        *transaction = None;
        Ok(RoutePreparation::Preserved)
    } else {
        Ok(RoutePreparation::Temporary)
    }
}

/// A removed original endpoint can return and Windows can already have restored
/// every role. Confirm the complete original identity vector without any setter.
/// Callers retain the journal unless this read-only completion is fully proven.
fn complete_if_already_restored(
    backend: &mut impl RouteBackend,
    tx: &Transaction,
    check: impl Fn() -> Result<(), String>,
) -> Result<bool, String> {
    check()?;
    if tx.transition.is_some() || backend.roles()? != tx.original {
        return Ok(false);
    }
    for role in 0..3 {
        let id = tx.original[role].as_deref().ok_or("original_missing")?;
        if Some(backend.active_name(id)?) != tx.original_names[role] {
            return Err("original_changed".into());
        }
    }
    check()?;
    let actual = backend.roles()?;
    if actual != tx.original {
        return Ok(false);
    }
    backend.persist(None)?;
    backend.observe("already_restored", tx, &actual);
    Ok(true)
}

fn restore_checked(
    backend: &mut impl RouteBackend,
    tx: &mut Transaction,
    check: impl Fn() -> Result<(), String>,
) -> Result<(), String> {
    check()?;
    if tx.transition.is_some() {
        let actual = backend.roles()?;
        validate_recovery(tx, &actual).map_err(|_| "external_change".to_owned())?;
        confirm_actual(tx, actual);
        backend.persist(Some(tx))?;
    }
    // ANY external role change relinquishes the WHOLE transaction. In particular,
    // do not restore the remaining roles after a user has changed one of them.
    check_expected(backend, tx)?;
    for role in 0..3 {
        if !tx.changed[role] || tx.expected[role].as_deref() != Some(&tx.target) {
            continue;
        }
        let id = tx.original[role].as_deref().ok_or("original_missing")?;
        if Some(backend.active_name(id)?) != tx.original_names[role] {
            return Err("original_changed".into());
        }
        let id = id.to_owned();
        set_group(backend, tx, role, &id, &check)?;
    }
    check()?;
    check_expected(backend, tx)?;
    if tx.expected != tx.original {
        return Err("restore_not_confirmed".into());
    }
    backend.observe("restored", tx, &tx.expected);
    backend.persist(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    struct Fake {
        actual: Roles,
        writes: Vec<(usize, String)>,
        durable: Option<Transaction>,
        conflict_after: Option<usize>,
        cancel: std::rc::Rc<Cell<bool>>,
        cancel_after: Option<usize>,
        fail_set_after: Option<usize>,
        fail_persist: bool,
        fail_clear: bool,
        cancel_on_clear: bool,
        journal_writes: usize,
        coupled: bool,
        missing: Option<String>,
        read_error: bool,
        role_reads: usize,
        cancel_on_role_read: Option<usize>,
    }
    impl RouteBackend for Fake {
        fn roles(&mut self) -> Result<Roles, String> {
            self.role_reads += 1;
            if self.cancel_on_role_read == Some(self.role_reads) {
                self.cancel.set(true);
            }
            if self.read_error {
                return Err("default_read_failed".into());
            }
            Ok(self.actual.clone())
        }
        fn active_name(&mut self, id: &str) -> Result<String, String> {
            if self.missing.as_deref() == Some(id) {
                return Err("capture_endpoint_missing".into());
            }
            Ok(id.to_owned())
        }
        fn set(&mut self, role: usize, id: &str) -> Result<(), String> {
            let transition = self.durable.as_ref().unwrap().transition.as_ref().unwrap();
            assert!(transition.mask & (1 << role) != 0);
            self.actual[role] = Some(id.into());
            if self.coupled && role < 2 {
                self.actual[1 - role] = Some(id.into());
            }
            self.writes.push((role, id.into()));
            if self.conflict_after == Some(self.writes.len()) {
                self.actual[2] = Some("external".into());
            }
            if self.cancel_after == Some(self.writes.len()) {
                self.cancel.set(true);
            }
            if self.fail_set_after == Some(self.writes.len()) {
                return Err("set_failed_after_write".into());
            }
            Ok(())
        }
        fn persist(&mut self, value: Option<&Transaction>) -> Result<(), String> {
            self.journal_writes += 1;
            if self.fail_persist || (value.is_none() && self.fail_clear) {
                return Err("journal_failed".into());
            }
            self.durable = value.cloned();
            if value.is_none() && self.cancel_on_clear {
                self.cancel.set(true);
            }
            Ok(())
        }
    }
    #[test]
    fn manual_defaults_preserve_success_without_a_session_lease() {
        let (mut io, _) = fixture();
        manual_default(&mut io, "target".into(), "target".into(), || Ok(())).unwrap();
        assert_eq!(
            io.actual,
            [
                Some("target".into()),
                Some("target".into()),
                Some("target".into())
            ]
        );
        assert!(io.durable.is_none());
        io.writes.clear();
        io.journal_writes = 0;
        manual_default(&mut io, "target".into(), "target".into(), || Ok(())).unwrap();
        assert!(io.writes.is_empty());
        assert_eq!(io.journal_writes, 0);
    }
    #[test]
    fn manual_defaults_cancel_after_write_rolls_back() {
        let (mut io, tx) = fixture();
        io.cancel_after = Some(1);
        let flag = io.cancel.clone();
        assert!(manual_default(
            &mut io,
            "target".into(),
            "target".into(),
            || if flag.get() {
                Err("cancelled".into())
            } else {
                Ok(())
            }
        )
        .is_err());
        assert_eq!(io.actual, tx.original);
        assert!(io.durable.is_none());
    }
    #[test]
    fn manual_defaults_reject_stale_name_and_split_roles_without_writes() {
        let (mut io, _) = fixture();
        assert!(manual_default(&mut io, "target".into(), "stale".into(), || Ok(())).is_err());
        io.actual[1] = Some("other".into());
        assert!(manual_default(&mut io, "target".into(), "target".into(), || Ok(())).is_err());
        assert!(io.writes.is_empty());
    }
    #[test]
    fn manual_defaults_failures_restore_partial_writes_and_preserve_external_choice() {
        let (mut io, tx) = fixture();
        io.fail_set_after = Some(1);
        assert!(manual_default(&mut io, "target".into(), "target".into(), || Ok(())).is_err());
        assert_eq!(io.actual, tx.original);
        assert!(io.durable.is_none());
        let (mut io, _) = fixture();
        io.conflict_after = Some(1);
        assert!(manual_default(&mut io, "target".into(), "target".into(), || Ok(())).is_err());
        assert_eq!(io.actual[2].as_deref(), Some("external"));
        assert_eq!(io.writes.len(), 1);
        assert!(io.durable.is_none());
    }
    #[test]
    fn manual_defaults_expired_request_never_writes() {
        let (mut io, _) = fixture();
        assert_eq!(
            manual_default(&mut io, "target".into(), "target".into(), || Err(
                "route_operation_expired".into()
            )),
            Err("route_operation_expired".into())
        );
        assert!(io.writes.is_empty());
        assert_eq!(io.journal_writes, 0);
    }
    fn manual_default(
        backend: &mut impl RouteBackend,
        target: String,
        name: String,
        check: impl Fn() -> Result<(), String>,
    ) -> Result<(), String> {
        super::manual_default(42, backend, target, name, check)
    }
    fn fixture() -> (Fake, Transaction) {
        let actual = [Some("a".into()), Some("a".into()), Some("c".into())];
        let tx = Transaction {
            version: 1,
            generation: 7,
            target: "target".into(),
            target_name: "target".into(),
            original: actual.clone(),
            original_names: actual.clone(),
            expected: actual.clone(),
            changed: [false; 3],
            transition: None,
        };
        (
            Fake {
                actual,
                writes: vec![],
                durable: None,
                conflict_after: None,
                cancel: Default::default(),
                cancel_after: None,
                fail_set_after: None,
                fail_persist: false,
                fail_clear: false,
                cancel_on_clear: false,
                journal_writes: 0,
                coupled: false,
                missing: None,
                read_error: false,
                role_reads: 0,
                cancel_on_role_read: None,
            },
            tx,
        )
    }
    #[test]
    fn already_default_has_no_journal_or_default_writes() {
        let (mut io, mut tx) = fixture();
        io.actual = std::array::from_fn(|_| Some(tx.target.clone()));
        tx.original = io.actual.clone();
        tx.original_names = io.actual.clone();
        tx.expected = io.actual.clone();
        apply(&mut io, &mut tx, || false).unwrap();
        assert!(io.writes.is_empty());
        assert_eq!(io.journal_writes, 0);
        assert!(io.durable.is_none());
    }
    #[test]
    fn already_default_still_checks_identity_and_cancellation() {
        let (mut io, mut tx) = fixture();
        io.actual = std::array::from_fn(|_| Some(tx.target.clone()));
        tx.original = io.actual.clone();
        tx.original_names = io.actual.clone();
        tx.expected = io.actual.clone();
        tx.target_name = "renamed".into();
        assert_eq!(
            apply(&mut io, &mut tx, || false),
            Err("target_changed".into())
        );
        tx.target_name = tx.target.clone();
        assert_eq!(apply(&mut io, &mut tx, || true), Err("cancelled".into()));
        assert_eq!(io.journal_writes, 0);
    }
    #[test]
    fn preserved_success_clears_journal_without_restoring_defaults() {
        let (mut io, mut tx) = fixture();
        apply(&mut io, &mut tx, || false).unwrap();
        let writes = io.writes.clone();
        preserve(&mut io, &tx, || false).unwrap();
        assert!(io.durable.is_none());
        assert_eq!(io.actual, tx.expected);
        assert_eq!(io.writes, writes);
    }
    #[test]
    fn preserved_clear_failure_retains_transaction_for_rollback() {
        let (mut io, mut tx) = fixture();
        apply(&mut io, &mut tx, || false).unwrap();
        io.fail_clear = true;
        assert_eq!(
            preserve(&mut io, &tx, || false),
            Err("journal_failed".into())
        );
        assert_eq!(io.durable, Some(tx.clone()));
        io.fail_clear = false;
        restore(&mut io, &mut tx).unwrap();
        assert_eq!(io.actual, tx.original);
    }
    #[test]
    fn preserved_clear_cancellation_still_rolls_back_completed_writes() {
        let (mut io, mut tx) = fixture();
        apply(&mut io, &mut tx, || false).unwrap();
        io.cancel_on_clear = true;
        let cancel = io.cancel.clone();
        assert_eq!(
            preserve(&mut io, &tx, || cancel.get()),
            Err("cancelled".into())
        );
        io.cancel_on_clear = false;
        restore(&mut io, &mut tx).unwrap();
        assert_eq!(io.actual, tx.original);
        assert!(io.durable.is_none());
    }
    #[test]
    fn preserved_completion_rejects_external_change_without_writing_roles() {
        let (mut io, mut tx) = fixture();
        apply(&mut io, &mut tx, || false).unwrap();
        io.actual[2] = Some("external".into());
        let before = io.actual.clone();
        let writes = io.writes.clone();
        assert_eq!(
            preserve(&mut io, &tx, || false),
            Err("external_change".into())
        );
        assert_eq!(io.actual, before);
        assert_eq!(io.writes, writes);
        assert!(io.durable.is_some());
    }
    #[test]
    fn failed_partial_switch_remains_recoverable_before_preservation() {
        let (mut io, mut tx) = fixture();
        io.fail_set_after = Some(1);
        assert_eq!(
            apply(&mut io, &mut tx, || false),
            Err("set_failed_after_write".into())
        );
        assert!(io.durable.is_some());
        restore(&mut io, &mut tx).unwrap();
        assert_eq!(io.actual, tx.original);
        assert!(io.durable.is_none());
    }
    #[test]
    fn preserved_completion_requires_successful_roles_readback() {
        let (mut io, mut tx) = fixture();
        apply(&mut io, &mut tx, || false).unwrap();
        io.read_error = true;
        assert_eq!(
            preserve(&mut io, &tx, || false),
            Err("default_read_failed".into())
        );
        assert!(io.durable.is_some());
        io.read_error = false;
        restore(&mut io, &mut tx).unwrap();
        assert_eq!(io.actual, tx.original);
    }
    #[test]
    fn preparation_returns_no_lease_for_already_default_in_both_modes() {
        for keep_selected in [false, true] {
            let (mut io, mut tx) = fixture();
            io.actual = std::array::from_fn(|_| Some(tx.target.clone()));
            tx.original = io.actual.clone();
            tx.original_names = io.actual.clone();
            tx.expected = io.actual.clone();
            let mut transaction = Some(tx);
            assert_eq!(
                prepare_route(&mut io, &mut transaction, keep_selected, || false),
                Ok(RoutePreparation::AlreadyDefault)
            );
            assert!(transaction.is_none());
            assert!(io.writes.is_empty());
            assert_eq!(io.journal_writes, 0);
        }
    }
    #[test]
    fn cancelled_already_default_preparation_has_no_lease_or_journal_to_restore() {
        let (mut io, mut tx) = fixture();
        io.actual = std::array::from_fn(|_| Some(tx.target.clone()));
        tx.original = io.actual.clone();
        tx.original_names = io.actual.clone();
        tx.expected = io.actual.clone();
        let mut transaction = Some(tx);
        assert_eq!(
            prepare_route(&mut io, &mut transaction, true, || true),
            Err("cancelled".into())
        );
        assert!(transaction.is_none());
        assert!(io.writes.is_empty());
        assert_eq!(io.journal_writes, 0);
    }
    #[test]
    fn external_change_during_already_default_preparation_has_no_journal_or_lease() {
        let (mut io, mut tx) = fixture();
        io.actual = std::array::from_fn(|_| Some(tx.target.clone()));
        tx.original = io.actual.clone();
        tx.original_names = io.actual.clone();
        tx.expected = io.actual.clone();
        io.actual[2] = Some("external".into());
        let mut transaction = Some(tx);
        assert_eq!(
            prepare_route(&mut io, &mut transaction, false, || false),
            Err("external_change".into())
        );
        assert!(transaction.is_none());
        assert!(io.writes.is_empty());
        assert_eq!(io.journal_writes, 0);
    }
    #[test]
    fn temporary_preparation_keeps_the_original_lease() {
        let (mut io, tx) = fixture();
        let mut transaction = Some(tx);
        assert_eq!(
            prepare_route(&mut io, &mut transaction, false, || false),
            Ok(RoutePreparation::Temporary)
        );
        let mut tx = transaction.unwrap();
        assert_eq!(io.durable, Some(tx.clone()));
        restore(&mut io, &mut tx).unwrap();
        assert_eq!(io.actual, tx.original);
    }
    #[test]
    fn committed_preservation_has_no_lease_for_later_release_or_shutdown() {
        let (mut io, tx) = fixture();
        let expected = std::array::from_fn(|_| Some(tx.target.clone()));
        let mut transaction = Some(tx);
        let cancel = io.cancel.clone();
        assert_eq!(
            prepare_route(&mut io, &mut transaction, true, || cancel.get()),
            Ok(RoutePreparation::Preserved)
        );
        cancel.set(true);
        assert!(transaction.is_none());
        assert!(io.durable.is_none());
        assert_eq!(io.actual, expected);
    }
    #[test]
    fn failed_preservation_preparation_keeps_the_original_rollback_transaction() {
        for cancel_on_clear in [false, true] {
            let (mut io, tx) = fixture();
            io.fail_clear = !cancel_on_clear;
            io.cancel_on_clear = cancel_on_clear;
            let cancel = io.cancel.clone();
            let mut transaction = Some(tx);
            assert!(prepare_route(&mut io, &mut transaction, true, || cancel.get()).is_err());
            let mut tx = transaction.unwrap();
            io.fail_clear = false;
            io.cancel_on_clear = false;
            restore(&mut io, &mut tx).unwrap();
            assert_eq!(io.actual, tx.original);
            assert!(io.durable.is_none());
        }
    }
    #[test]
    fn cancellation_during_last_role_read_never_reaches_the_setter() {
        let (mut io, mut tx) = fixture();
        io.cancel_on_role_read = Some(3);
        let cancel = io.cancel.clone();
        assert_eq!(
            apply(&mut io, &mut tx, || cancel.get()),
            Err("cancelled".into())
        );
        assert!(io.writes.is_empty());
        restore(&mut io, &mut tx).unwrap();
        assert_eq!(io.actual, tx.original);
        assert!(io.durable.is_none());
    }
    #[test]
    fn returned_original_vector_completes_journal_without_a_setter() {
        let (mut io, mut tx) = fixture();
        apply(&mut io, &mut tx, || false).unwrap();
        io.missing = Some("a".into());
        assert_eq!(
            restore(&mut io, &mut tx).unwrap_err(),
            "capture_endpoint_missing"
        );
        let writes = io.writes.len();
        io.missing = None;
        io.actual = tx.original.clone();
        assert!(complete_if_already_restored(&mut io, &tx, || Ok(())).unwrap());
        assert_eq!(io.writes.len(), writes);
        assert!(io.durable.is_none());
    }
    #[test]
    fn zero_write_completion_never_accepts_target_mixed_or_external_roles() {
        let (mut io, mut tx) = fixture();
        apply(&mut io, &mut tx, || false).unwrap();
        let durable = io.durable.clone();
        let writes = io.writes.len();
        for actual in [
            tx.expected.clone(),
            [
                tx.original[0].clone(),
                tx.original[1].clone(),
                Some("target".into()),
            ],
            [
                tx.original[0].clone(),
                tx.original[1].clone(),
                Some("external".into()),
            ],
        ] {
            io.actual = actual.clone();
            assert!(!complete_if_already_restored(&mut io, &tx, || Ok(())).unwrap());
            assert_eq!(io.actual, actual);
            assert_eq!(io.durable, durable);
            assert_eq!(io.writes.len(), writes);
        }
    }
    #[test]
    fn zero_write_completion_retains_record_on_endpoint_read_journal_or_cancel_failure() {
        let (mut io, mut tx) = fixture();
        apply(&mut io, &mut tx, || false).unwrap();
        io.actual = tx.original.clone();
        let durable = io.durable.clone();
        let writes = io.writes.len();
        io.missing = Some("a".into());
        assert_eq!(
            complete_if_already_restored(&mut io, &tx, || Ok(())).unwrap_err(),
            "capture_endpoint_missing"
        );
        io.missing = None;
        io.read_error = true;
        assert_eq!(
            complete_if_already_restored(&mut io, &tx, || Ok(())).unwrap_err(),
            "default_read_failed"
        );
        io.read_error = false;
        io.fail_persist = true;
        assert_eq!(
            complete_if_already_restored(&mut io, &tx, || Ok(())).unwrap_err(),
            "journal_failed"
        );
        io.fail_persist = false;
        assert_eq!(
            complete_if_already_restored(&mut io, &tx, || Err("cancelled".into())).unwrap_err(),
            "cancelled"
        );
        tx.original_names[0] = Some("renamed".into());
        assert_eq!(
            complete_if_already_restored(&mut io, &tx, || Ok(())).unwrap_err(),
            "original_changed"
        );
        assert_eq!(io.durable, durable);
        assert_eq!(io.writes.len(), writes);
    }
    #[test]
    fn zero_write_completion_does_not_bypass_an_unresolved_setter_transition() {
        let (mut io, mut tx) = fixture();
        tx.transition = Some(Transition {
            role: 0,
            mask: 3,
            before: tx.original.clone(),
            desired: [
                Some("target".into()),
                Some("target".into()),
                tx.original[2].clone(),
            ],
        });
        io.durable = Some(tx.clone());
        assert!(!complete_if_already_restored(&mut io, &tx, || Ok(())).unwrap());
        assert_eq!(io.durable, Some(tx));
        assert!(io.writes.is_empty());
    }
    #[test]
    fn normal_session_restores_only_its_changes() {
        let (mut io, mut tx) = fixture();
        io.actual[2] = Some("target".into());
        tx.original = io.actual.clone();
        tx.expected = io.actual.clone();
        tx.original_names = io.actual.clone();
        apply(&mut io, &mut tx, || false).unwrap();
        restore(&mut io, &mut tx).unwrap();
        assert_eq!(io.actual, tx.original);
        assert_eq!(io.writes.len(), 4);
        assert!(io.durable.is_none());
    }
    #[test]
    fn external_change_relinquishes_every_role() {
        let (mut io, mut tx) = fixture();
        apply(&mut io, &mut tx, || false).unwrap();
        io.actual[1] = Some("user".into());
        let before = io.actual.clone();
        assert_eq!(restore(&mut io, &mut tx).unwrap_err(), "external_change");
        assert_eq!(io.actual, before);
        assert_eq!(io.writes.len(), 3);
    }
    #[test]
    fn conflict_during_apply_never_writes_remaining_roles() {
        let (mut io, mut tx) = fixture();
        io.conflict_after = Some(1);
        assert_eq!(
            apply(&mut io, &mut tx, || false).unwrap_err(),
            "external_change"
        );
        assert_eq!(io.writes.len(), 1);
    }
    #[test]
    fn release_inside_setter_rolls_back_without_late_activation() {
        let (mut io, mut tx) = fixture();
        io.cancel_after = Some(1);
        let cancel = io.cancel.clone();
        assert_eq!(
            apply(&mut io, &mut tx, || cancel.get()).unwrap_err(),
            "cancelled"
        );
        restore(&mut io, &mut tx).unwrap();
        assert_eq!(io.actual, tx.original);
        assert_eq!(io.writes.len(), 2);
    }
    #[test]
    fn journal_failure_and_already_released_never_change_defaults() {
        let (mut io, mut tx) = fixture();
        io.fail_persist = true;
        assert!(apply(&mut io, &mut tx, || false).is_err());
        assert!(io.writes.is_empty());
        io.fail_persist = false;
        assert!(apply(&mut io, &mut tx, || true).is_err());
        assert!(io.writes.is_empty());
    }
    #[test]
    fn conflict_during_restore_stops_all_remaining_writes() {
        let (mut io, mut tx) = fixture();
        apply(&mut io, &mut tx, || false).unwrap();
        io.conflict_after = Some(4);
        assert_eq!(restore(&mut io, &mut tx).unwrap_err(), "external_change");
        assert_eq!(io.writes.len(), 4);
    }

    #[test]
    fn crash_recovery_rejects_external_change_to_preexisting_target_role() {
        let (mut io, mut tx) = fixture();
        io.actual[0] = Some("target".into());
        io.actual[1] = Some("target".into());
        tx.original = io.actual.clone();
        tx.original_names = io.actual.clone();
        tx.expected = io.actual.clone();
        apply(&mut io, &mut tx, || false).unwrap();
        let journal = io.durable.clone().unwrap();
        assert!(!journal.changed[0]);
        assert!(validate_recovery(&journal, &io.actual).is_ok());
        io.actual[0] = Some("user_or_headset".into());
        let writes_before = io.writes.len();
        let before = io.actual.clone();
        let result = validate_recovery(&journal, &io.actual);
        assert_eq!(result.unwrap_err(), "recovery_external_change");
        assert_eq!(io.writes.len(), writes_before);
        assert_eq!(io.actual, before);
    }
    #[test]
    fn coupled_normal_roles_first_session_restores_both() {
        let (mut io, mut tx) = fixture();
        io.coupled = true;
        apply(&mut io, &mut tx, || false).unwrap();
        assert_eq!(io.writes.len(), 2);
        assert_eq!(tx.changed, [true; 3]);
        restore(&mut io, &mut tx).unwrap();
        assert_eq!(io.actual, tx.original);
        assert_eq!(io.writes.len(), 4);
        assert!(io.durable.is_none());
    }
    #[test]
    fn split_normal_roles_refused_before_any_write_or_journal() {
        let (mut io, mut tx) = fixture();
        io.actual[1] = Some("split".into());
        tx.original = io.actual.clone();
        tx.expected = io.actual.clone();
        assert_eq!(
            apply(&mut io, &mut tx, || false).unwrap_err(),
            "normal_roles_split"
        );
        assert!(io.writes.is_empty());
        assert!(io.durable.is_none());
    }
    #[test]
    fn coupled_setter_never_accepts_external_communications_change() {
        let (mut io, mut tx) = fixture();
        io.coupled = true;
        io.conflict_after = Some(1);
        assert_eq!(
            apply(&mut io, &mut tx, || false).unwrap_err(),
            "external_change"
        );
        assert_eq!(io.writes.len(), 1);
        assert_eq!(io.actual[2].as_deref(), Some("external"));
    }
    #[test]
    fn released_inside_coupled_setter_restores_without_second_voice_press() {
        let (mut io, mut tx) = fixture();
        io.coupled = true;
        io.cancel_after = Some(1);
        let cancel = io.cancel.clone();
        assert_eq!(
            apply(&mut io, &mut tx, || cancel.get()).unwrap_err(),
            "cancelled"
        );
        restore(&mut io, &mut tx).unwrap();
        assert_eq!(io.actual, tx.original);
        assert_eq!(io.writes.len(), 2);
    }
    #[test]
    fn inflight_crash_before_partial_or_coupled_setter_recovers_exact_original() {
        for changed in [0u8, 1, 2, 3] {
            let (mut io, mut tx) = fixture();
            io.coupled = true;
            let mut desired = tx.expected.clone();
            desired[0] = Some(tx.target.clone());
            desired[1] = Some(tx.target.clone());
            tx.transition = Some(Transition {
                role: 0,
                mask: 3,
                before: tx.expected.clone(),
                desired,
            });
            io.persist(Some(&tx)).unwrap();
            for i in 0..2 {
                if changed & (1 << i) != 0 {
                    io.actual[i] = Some(tx.target.clone());
                }
            }
            assert!(validate_recovery(&tx, &io.actual).is_ok());
            restore(&mut io, &mut tx).unwrap();
            assert_eq!(io.actual, tx.original);
            assert!(io.durable.is_none());
        }
    }
    #[test]
    fn inflight_recovery_rejects_third_value_and_outside_group_with_zero_writes() {
        for role in 0..3 {
            let (mut io, mut tx) = fixture();
            let mut desired = tx.expected.clone();
            desired[0] = Some(tx.target.clone());
            desired[1] = Some(tx.target.clone());
            tx.transition = Some(Transition {
                role: 0,
                mask: 3,
                before: tx.expected.clone(),
                desired,
            });
            io.actual[role] = Some("external".into());
            assert!(validate_recovery(&tx, &io.actual).is_err());
            assert_eq!(restore(&mut io, &mut tx).unwrap_err(), "external_change");
            assert!(io.writes.is_empty());
        }
    }
}

// Rust guideline compliant 2026-02-21

//! Private write collector delegates all native operations to InMemoryStore.

use super::RuntimeError;
use super::staging::{
    NativeIntakeChange, NativePreparationError, NativeProcessChange, NativeRuntimeSnapshot,
};
use crate::store::{InMemoryStore, IntakeRecord, RuntimeRecord, RuntimeStore, StoreError};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, MutexGuard};
use wos_core::ProvenanceRecord;

type IntakeKey = (String, String);
struct Collected {
    store: InMemoryStore,
    before_processes: BTreeMap<String, RuntimeRecord>,
    before_intakes: BTreeMap<IntakeKey, IntakeRecord>,
    qualified_cases: BTreeSet<String>,
    queried_cases: BTreeSet<String>,
    written_processes: BTreeSet<String>,
    written_intakes: BTreeSet<IntakeKey>,
    #[cfg(test)]
    fault_at_write: Option<usize>,
    #[cfg(test)]
    write_count: usize,
}
#[derive(Clone)]
pub(super) struct StageStore(Arc<Mutex<Collected>>);
impl StageStore {
    pub(super) fn restore(snapshot: NativeRuntimeSnapshot) -> Result<Self, RuntimeError> {
        let invalid = |reason: &str| {
            RuntimeError::NativePreparation(NativePreparationError::InvalidSnapshot {
                reason: reason.into(),
            })
        };
        let mut records = BTreeMap::new();
        for record in snapshot.process_records {
            let key = record.process.process_id.clone();
            if records.insert(key, record).is_some() {
                return Err(invalid("duplicate process key"));
            }
        }
        let before_processes = records.clone();
        let mut store = InMemoryStore::new();
        let mut qualified_cases = BTreeSet::new();
        for membership in snapshot.complete_case_membership {
            if !stack_common_typeid::is_valid_type_id(
                &membership.case_ledger_id,
                Some(stack_common_typeid::CASE_PREFIX),
            ) {
                return Err(invalid("membership case is not a canonical case TypeID"));
            }
            if !qualified_cases.insert(membership.case_ledger_id.clone()) {
                return Err(invalid("duplicate case membership"));
            }
            let mut seen = BTreeSet::new();
            for process_id in &membership.process_ids {
                if !seen.insert(process_id.clone()) {
                    return Err(invalid("duplicate membership process"));
                }
                let record = records.remove(process_id).ok_or_else(|| {
                    invalid("membership process was not loaded or occurs in another case")
                })?;
                if record.process.case_ledger_id != membership.case_ledger_id {
                    return Err(invalid("loaded process belongs to a different case"));
                }
                store.restore_record(record)?;
            }
            if store.processes_for_case(&membership.case_ledger_id) != membership.process_ids {
                return Err(invalid(
                    "native restored case index differs from supplied membership",
                ));
            }
        }
        if !records.is_empty() {
            return Err(invalid(
                "loaded process omitted from complete case membership",
            ));
        }
        let mut before_intakes = BTreeMap::new();
        for record in snapshot.intake_records {
            let key = (record.binding.clone(), record.intake_id.clone());
            if before_intakes.insert(key, record.clone()).is_some() {
                return Err(invalid("duplicate intake key"));
            }
            store.create_intake_record(record)?;
        }
        Ok(Self(Arc::new(Mutex::new(Collected {
            store,
            before_processes,
            before_intakes,
            qualified_cases,
            queried_cases: BTreeSet::new(),
            written_processes: BTreeSet::new(),
            written_intakes: BTreeSet::new(),
            #[cfg(test)]
            fault_at_write: None,
            #[cfg(test)]
            write_count: 0,
        }))))
    }
    #[cfg(test)]
    pub(super) fn set_fault_at_write(&self, fault: Option<usize>) -> Result<(), StoreError> {
        self.lock()?.fault_at_write = fault;
        Ok(())
    }
    fn lock(&self) -> Result<MutexGuard<'_, Collected>, StoreError> {
        self.0
            .lock()
            .map_err(|_| StoreError::Failed("native stage collector lock poisoned".into()))
    }
    pub(super) fn validate_queries(&self) -> Result<(), RuntimeError> {
        let collected = self.lock()?;
        if let Some(case) = collected
            .queried_cases
            .difference(&collected.qualified_cases)
            .next()
        {
            return Err(NativePreparationError::MissingMembership {
                case_ledger_id: case.clone(),
            }
            .into());
        }
        Ok(())
    }
    pub(super) fn deferred_outbound_events(
        &self,
    ) -> Result<Vec<super::staging::DeferredOutboundEvent>, RuntimeError> {
        let collected = self.lock()?;
        let mut outbound = Vec::new();
        for process_id in &collected.written_processes {
            let after = collected.store.load_record(process_id)?;
            let before = collected.before_processes.get(process_id);
            let mut pending: Vec<_> = after.process.pending_outbound_events.iter().collect();
            pending.sort_by(|a, b| a.0.cmp(b.0));
            for (subject, event) in pending {
                match before.and_then(|record| record.process.pending_outbound_events.get(subject))
                {
                    Some(previous) if previous == event => {}
                    Some(_) => {
                        return Err(NativePreparationError::InvalidSnapshot {
                            reason: "retained deferred request changed".into(),
                        }
                        .into());
                    }
                    None => outbound.push(super::staging::DeferredOutboundEvent {
                        process_id: process_id.clone(),
                        event: event.clone(),
                    }),
                }
            }
        }
        Ok(outbound)
    }

    pub(super) fn finish(
        &self,
    ) -> Result<
        (
            Vec<NativeProcessChange>,
            Vec<NativeIntakeChange>,
            Vec<String>,
        ),
        RuntimeError,
    > {
        self.validate_queries()?;
        let collected = self.lock()?;
        let mut processes = vec![];
        let mut intakes = vec![];
        for key in &collected.written_processes {
            let after = collected.store.load_record(key)?;
            processes.push(match collected.before_processes.get(key) {
                Some(before) => NativeProcessChange::Replace {
                    before: before.clone(),
                    after,
                },
                None => NativeProcessChange::Create(after),
            });
        }
        for key in &collected.written_intakes {
            let after = collected.store.load_intake_record(&key.0, &key.1)?;
            intakes.push(match collected.before_intakes.get(key) {
                Some(before) => NativeIntakeChange::Replace {
                    before: before.clone(),
                    after,
                },
                None => NativeIntakeChange::Create(after),
            });
        }
        Ok((
            processes,
            intakes,
            collected.queried_cases.iter().cloned().collect(),
        ))
    }
}
impl Collected {
    fn before_write(&mut self) -> Result<(), StoreError> {
        #[cfg(test)]
        {
            self.write_count += 1;
            if self.fault_at_write == Some(self.write_count) {
                return Err(StoreError::Failed(
                    "injected speculative native write failure".into(),
                ));
            }
        }
        Ok(())
    }
}

impl RuntimeStore for StageStore {
    fn create_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
        let key = record.process.process_id.clone();
        let case = record.process.case_ledger_id.clone();
        let mut collected = self.lock()?;
        collected.queried_cases.insert(case);
        collected.before_write()?;
        collected.store.create_record(record)?;
        collected.written_processes.insert(key);
        Ok(())
    }
    fn load_record(&self, process_id: &str) -> Result<RuntimeRecord, StoreError> {
        self.lock()?.store.load_record(process_id)
    }
    fn processes_for_case(&self, case_ledger_id: &str) -> Vec<String> {
        let Ok(mut collected) = self.lock() else {
            return vec![];
        };
        collected.queried_cases.insert(case_ledger_id.into());
        collected.store.processes_for_case(case_ledger_id)
    }
    fn provenance_for_case(&self, case_ledger_id: &str) -> Vec<ProvenanceRecord> {
        let Ok(mut collected) = self.lock() else {
            return vec![];
        };
        collected.queried_cases.insert(case_ledger_id.into());
        collected.store.provenance_for_case(case_ledger_id)
    }
    fn append_provenance_for_case(
        &mut self,
        case_ledger_id: &str,
        process_id: &str,
        record: ProvenanceRecord,
    ) -> Result<(), StoreError> {
        let mut collected = self.lock()?;
        collected.queried_cases.insert(case_ledger_id.into());
        collected.before_write()?;
        collected
            .store
            .append_provenance_for_case(case_ledger_id, process_id, record)?;
        collected.written_processes.insert(process_id.into());
        Ok(())
    }
    fn save_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
        let key = record.process.process_id.clone();
        let mut collected = self.lock()?;
        collected.before_write()?;
        collected.store.save_record(record)?;
        collected.written_processes.insert(key);
        Ok(())
    }
    fn create_intake_record(&mut self, record: IntakeRecord) -> Result<(), StoreError> {
        let key = (record.binding.clone(), record.intake_id.clone());
        let mut collected = self.lock()?;
        collected.before_write()?;
        collected.store.create_intake_record(record)?;
        collected.written_intakes.insert(key);
        Ok(())
    }
    fn load_intake_record(
        &self,
        binding: &str,
        intake_id: &str,
    ) -> Result<IntakeRecord, StoreError> {
        self.lock()?.store.load_intake_record(binding, intake_id)
    }
    fn save_intake_record(&mut self, record: IntakeRecord) -> Result<(), StoreError> {
        let key = (record.binding.clone(), record.intake_id.clone());
        let mut collected = self.lock()?;
        collected.before_write()?;
        collected.store.save_intake_record(record)?;
        collected.written_intakes.insert(key);
        Ok(())
    }
}

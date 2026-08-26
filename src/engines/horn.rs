//! Horn hybrid: PP purity first, then bounded clause search (parity sketch).

use crate::engines::dispatch::EngineOutcome;
use crate::engines::partition::TuplePartition;
use crate::engines::types::atomic_pp_type;
use crate::first_order::models::Model;
use crate::first_order::relops::Relation;

pub fn check_horn(
    model: &Model,
    target: &Relation,
    max_depth: usize,
) -> Result<EngineOutcome, String> {
    let d = max_depth.max(1);
    let mut partition = TuplePartition::from_model(model, target.arity)?;
    partition.refine(|row| {
        let sig = atomic_pp_type(model, row, d);
        sig.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<_>>()
    });
    // Same as Python: if PP-pure, Horn-definable via that certificate.
    let definable = partition.is_target_pure(target);
    Ok(EngineOutcome {
        definable,
        fragment: "horn".into(),
        engine: format!("horn_pp_d{d}"),
    })
}

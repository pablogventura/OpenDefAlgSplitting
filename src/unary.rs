//! Backend unario: decisión barata de QF-definibilidad para targets de aridad 1.
//!
//! Justificación Lean (`PolyTime.qfDefinable_unary_noFunctions` /
//! `Positive.positiveDefinable_unary_noFunctions`):
//! sin operaciones de aridad > 0, R ⊆ A es QF-definible (y positivamente
//! definible) sii R = ∅ o R = A.
//!
//! Con operaciones unarias, se usa conteo de huellas del subuniverso generado
//! (esquema Castellano Cor. 13): para cada huella h de un a ∈ R debe valer
//! |R_h| = |A_h|. Soundness Lean: `SameTermEqFingerprint` + cobertura de
//! `T_{K_a}` ⇒ `not_qfDefinable_of_sameTermEqFingerprint_Ka_mix` (vía Lema mat
//! y `not_qfDefinable_of_approx_mix`).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::first_order::models::Model;
use crate::first_order::relops::{Operation, Relation};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnaryDecision {
    /// Definible: vacío o universo (sin ops), o conteo de huellas OK.
    Definable { reason: String },
    /// No definible: testigo opcional.
    NotDefinable { reason: String },
    /// No aplica este backend (aridad ≠ 1).
    Skip,
}

fn unary_ops(model: &Model) -> Vec<&Operation> {
    model
        .operations
        .values()
        .filter(|op| op.arity == 1)
        .collect()
}

fn has_nonunary_ops(model: &Model) -> bool {
    model.operations.values().any(|op| op.arity > 1)
}

/// Cierre del subuniverso generado por `seed` bajo operaciones unarias (+ constantes 0-arias).
fn generated_unary_closure(seed: i64, model: &Model) -> BTreeSet<i64> {
    let mut s = BTreeSet::new();
    s.insert(seed);
    for op in model.operations.values() {
        if op.arity == 0 {
            if let Some(v) = op.call(&[]) {
                s.insert(v);
            }
        }
    }
    let uops = unary_ops(model);
    let mut changed = true;
    while changed {
        changed = false;
        let snapshot: Vec<i64> = s.iter().copied().collect();
        for &x in &snapshot {
            for op in &uops {
                if let Some(y) = op.call(&[x]) {
                    if s.insert(y) {
                        changed = true;
                    }
                }
            }
        }
    }
    s
}

/// Huella canónica del álgebra unaria etiquetada generada por `seed`.
fn unary_fingerprint(seed: i64, model: &Model) -> Vec<u8> {
    let elems = generated_unary_closure(seed, model);
    let mut id_of: BTreeMap<i64, u32> = BTreeMap::new();
    // Generador primero.
    id_of.insert(seed, 0);
    let mut next = 1u32;
    for &e in &elems {
        id_of.entry(e).or_insert_with(|| {
            let i = next;
            next += 1;
            i
        });
    }
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(&(elems.len() as u32).to_le_bytes());
    let mut ops: Vec<&Operation> = model.operations.values().collect();
    ops.sort_by(|a, b| a.sym.cmp(&b.sym).then(a.arity.cmp(&b.arity)));
    for op in ops {
        out.extend_from_slice(&(op.arity as u32).to_le_bytes());
        out.extend_from_slice(op.sym.as_bytes());
        out.push(0);
        if op.arity == 0 {
            if let Some(v) = op.call(&[]) {
                if let Some(&id) = id_of.get(&v) {
                    out.extend_from_slice(&id.to_le_bytes());
                }
            }
        } else if op.arity == 1 {
            // Encode the op table in generator-id order so isomorphic
            // pointed unary algebras share a fingerprint (Castellano Cor.13).
            let mut by_id: Vec<(u32, i64)> = id_of.iter().map(|(&e, &id)| (id, e)).collect();
            by_id.sort_by_key(|(id, _)| *id);
            for &(_id, e) in &by_id {
                if let Some(v) = op.call(&[e]) {
                    if let (Some(&ie), Some(&iv)) = (id_of.get(&e), id_of.get(&v)) {
                        out.extend_from_slice(&ie.to_le_bytes());
                        out.extend_from_slice(&iv.to_le_bytes());
                    }
                }
            }
        }
    }
    out
}

/// Decide definibilidad QF para target de aridad 1, o `Skip` si no aplica.
pub fn decide_unary(model: &Model, target: &Relation) -> UnaryDecision {
    if target.arity != 1 {
        return UnaryDecision::Skip;
    }
    if has_nonunary_ops(model) {
        return UnaryDecision::Skip;
    }

    let u: HashSet<i64> = model.universe.iter().copied().collect();
    let in_t: HashSet<i64> = target
        .r
        .iter()
        .filter_map(|t| t.first().copied())
        .collect();

    let ops_gt0 = model.operations.values().any(|op| op.arity > 0);
    if !ops_gt0 {
        // Lean: qfDefinable_unary_noFunctions
        if in_t.is_empty() {
            return UnaryDecision::Definable {
                reason: "unary no-ops: R=∅ (Lean qfDefinable_unary_noFunctions)".into(),
            };
        }
        if in_t == u {
            return UnaryDecision::Definable {
                reason: "unary no-ops: R=A (Lean qfDefinable_unary_noFunctions)".into(),
            };
        }
        return UnaryDecision::NotDefinable {
            reason: "unary no-ops: R neither empty nor full".into(),
        };
    }

    // Conteo de huellas (Castellano Cor. 13, aridad 1).
    let mut count_all: HashMap<Vec<u8>, u64> = HashMap::new();
    let mut count_r: HashMap<Vec<u8>, u64> = HashMap::new();
    for &a in &model.universe {
        let fp = unary_fingerprint(a, model);
        *count_all.entry(fp.clone()).or_insert(0) += 1;
        if in_t.contains(&a) {
            *count_r.entry(fp).or_insert(0) += 1;
        }
    }
    for (fp, &cr) in &count_r {
        let ca = count_all.get(fp).copied().unwrap_or(0);
        if cr != ca {
            return UnaryDecision::NotDefinable {
                reason: format!(
                    "unary fingerprint mix: |R_h|={cr} != |A_h|={ca} (Castellano Cor.13)"
                ),
            };
        }
    }
    UnaryDecision::Definable {
        reason: "unary fingerprint counts match (Castellano Cor.13)".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn unary_no_ops_partial_not_definable() {
        let universe = vec![0, 1, 2];
        let target = Relation::new("T", 1).with_tuples(vec![vec![0], vec![1]]);
        let model = Model::new(universe, HashMap::new(), HashMap::new());
        match decide_unary(&model, &target) {
            UnaryDecision::NotDefinable { .. } => {}
            other => panic!("esperaba NotDefinable, got {:?}", other),
        }
    }

    #[test]
    fn unary_no_ops_full_definable() {
        let universe = vec![0, 1];
        let target = Relation::new("T", 1).with_tuples(vec![vec![0], vec![1]]);
        let model = Model::new(universe, HashMap::new(), HashMap::new());
        match decide_unary(&model, &target) {
            UnaryDecision::Definable { .. } => {}
            other => panic!("esperaba Definable, got {:?}", other),
        }
    }

    #[test]
    fn arity_two_skips() {
        let universe = vec![0, 1];
        let target = Relation::new("T", 2).with_tuples(vec![vec![0, 0]]);
        let model = Model::new(universe, HashMap::new(), HashMap::new());
        assert!(matches!(decide_unary(&model, &target), UnaryDecision::Skip));
    }

    /// Companion worked example (b): three elements, no ops, partial target.
    #[test]
    fn unary_three_element_partial_rejects() {
        let universe = vec![0, 1, 2];
        let target = Relation::new("T", 1).with_tuples(vec![vec![0], vec![1]]);
        let model = Model::new(universe, HashMap::new(), HashMap::new());
        match decide_unary(&model, &target) {
            UnaryDecision::NotDefinable { reason } => {
                assert!(
                    reason.contains("neither empty nor full"),
                    "expected no-ops partial reject, got {reason}"
                );
            }
            other => panic!("expected NotDefinable, got {:?}", other),
        }
    }

    #[test]
    fn unary_flip_partial_not_definable() {
        use crate::first_order::relops::Operation;
        let universe = vec![0, 1];
        let mut s = Operation::new("s", 1);
        s.add(vec![0, 1]);
        s.add(vec![1, 0]);
        let mut ops = HashMap::new();
        ops.insert("s".into(), s);
        let target = Relation::new("T0", 1).with_tuples(vec![vec![0]]);
        let model = Model::new(universe, HashMap::new(), ops);
        let fp0 = unary_fingerprint(0, &model);
        let fp1 = unary_fingerprint(1, &model);
        assert_eq!(fp0, fp1, "flip should give isomorphic fingerprints");
        match decide_unary(&model, &target) {
            UnaryDecision::NotDefinable { .. } => {}
            other => panic!("esperaba NotDefinable, got {:?}", other),
        }
    }
}

/// One row of unary-backend coverage for a single target relation.
#[derive(Clone, Debug)]
pub struct UnaryAuditRow {
    pub target_sym: String,
    pub arity: usize,
    pub decision: String,
    pub has_nonunary_ops: bool,
    pub would_early_exit: bool,
    pub reason: String,
}

fn decision_label(d: &UnaryDecision) -> &'static str {
    match d {
        UnaryDecision::Skip => "Skip",
        UnaryDecision::Definable { .. } => "Definable",
        UnaryDecision::NotDefinable { .. } => "NotDefinable",
    }
}

/// Classify every target relation (symbols starting with `T`) under `decide_unary`.
pub fn audit_unary_targets(model: &Model) -> Vec<UnaryAuditRow> {
    let has_nonunary = has_nonunary_ops(model);
    model
        .relations
        .iter()
        .filter(|(sym, _)| sym.starts_with('T'))
        .map(|(sym, rel)| {
            let decision = decide_unary(model, rel);
            let would_early_exit = matches!(
                &decision,
                UnaryDecision::Definable { .. } | UnaryDecision::NotDefinable { .. }
            );
            let reason = match &decision {
                UnaryDecision::Skip => "skip".into(),
                UnaryDecision::Definable { reason } | UnaryDecision::NotDefinable { reason } => {
                    reason.clone()
                }
            };
            UnaryAuditRow {
                target_sym: sym.clone(),
                arity: rel.arity,
                decision: decision_label(&decision).into(),
                has_nonunary_ops: has_nonunary,
                would_early_exit,
                reason,
            }
        })
        .collect()
}

//! Precheck de definibilidad QF vía clases ≈ aproximadas.
//!
//! Para cada tupla se cierra el subuniverso generado por sus coordenadas bajo las
//! operaciones y se construye una huella canónica del álgebra etiquetada (los
//! generadores son las coordenadas). Si dos tuplas tienen la misma huella y una
//! está en T y la otra no, T no es unión de clases ≈ y no es QF-definible.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::first_order::models::Model;
use crate::first_order::relops::{Operation, Relation};

/// True si `target` no puede ser QF-definible porque mezcla una clase ≈ aproximada.
pub fn target_breaks_approx_classes(model: &Model, target: &Relation) -> bool {
    let k = target.arity;
    if k == 0 {
        return false;
    }
    let ops: Vec<&Operation> = model.operations.values().collect();
    let mut by_fp: HashMap<Vec<u8>, (bool, bool)> = HashMap::new();
    for tuple in cartesian_product(&model.universe, k) {
        let in_t = target.contains(&tuple);
        let fp = labeled_subalgebra_fingerprint(&tuple, &ops);
        let entry = by_fp.entry(fp).or_insert((false, false));
        if in_t {
            entry.0 = true;
        } else {
            entry.1 = true;
        }
        if entry.0 && entry.1 {
            return true;
        }
    }
    false
}

fn labeled_subalgebra_fingerprint(gens: &[i64], ops: &[&Operation]) -> Vec<u8> {
    let mut elems: BTreeSet<i64> = gens.iter().copied().collect();
    let mut changed = true;
    while changed {
        changed = false;
        let snapshot: Vec<i64> = elems.iter().copied().collect();
        for op in ops {
            if op.arity == 0 {
                if let Some(v) = op.call(&[]) {
                    if elems.insert(v) {
                        changed = true;
                    }
                }
                continue;
            }
            for args in cartesian_product(&snapshot, op.arity) {
                if let Some(v) = op.call(&args) {
                    if elems.insert(v) {
                        changed = true;
                    }
                }
            }
        }
    }
    let mut id_of: BTreeMap<i64, u32> = BTreeMap::new();
    let mut next = 0u32;
    for &g in gens {
        if let std::collections::btree_map::Entry::Vacant(e) = id_of.entry(g) {
            e.insert(next);
            next += 1;
        }
    }
    for &e in &elems {
        id_of.entry(e).or_insert_with(|| {
            let i = next;
            next += 1;
            i
        });
    }
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(&(gens.len() as u32).to_le_bytes());
    out.extend_from_slice(&(elems.len() as u32).to_le_bytes());
    let mut ops_sorted: Vec<&Operation> = ops.to_vec();
    ops_sorted.sort_by(|a, b| a.sym.cmp(&b.sym).then(a.arity.cmp(&b.arity)));
    for op in ops_sorted {
        out.extend(op.sym.as_bytes());
        out.push(0);
        out.push(op.arity as u8);
        let domain: Vec<i64> = elems.iter().copied().collect();
        if op.arity == 0 {
            let v = op.call(&[]).unwrap_or(i64::MIN);
            out.extend_from_slice(&id_of.get(&v).copied().unwrap_or(u32::MAX).to_le_bytes());
            continue;
        }
        for args in cartesian_product(&domain, op.arity) {
            let v = op.call(&args).unwrap_or(i64::MIN);
            out.extend_from_slice(&id_of.get(&v).copied().unwrap_or(u32::MAX).to_le_bytes());
        }
    }
    for &g in gens {
        out.extend_from_slice(&id_of[&g].to_le_bytes());
    }
    out
}

fn cartesian_product(items: &[i64], n: usize) -> Vec<Vec<i64>> {
    if n == 0 {
        return vec![vec![]];
    }
    let mut result = vec![vec![]];
    for _ in 0..n {
        let mut next = Vec::new();
        for row in &result {
            for &item in items {
                let mut r = row.clone();
                r.push(item);
                next.push(r);
            }
        }
        result = next;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_model;
    use std::path::Path;

    #[test]
    fn suma4_approx_runs() {
        let p = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("model_examples")
            .join("suma4.model");
        if !p.exists() {
            return;
        }
        let model = parse_model(Some(&p), true).expect("parse");
        let t = model
            .relations
            .values()
            .find(|r| r.sym.starts_with('T'))
            .cloned()
            .expect("target");
        let _ = target_breaks_approx_classes(&model, &t);
    }
}

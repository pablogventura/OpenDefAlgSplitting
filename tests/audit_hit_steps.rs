use opendefalgsplitting::parser::parse_model;
use opendefalgsplitting::first_order::relops::Operation;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

// Replicate generator loop with logging by calling the public key fn many times
// and also use hit internals via a small hack: just parse model and call binary.

#[test]
fn count_steps_via_key_growth() {
    use opendefalgsplitting::hit::qf_isotype_equality_key;
    let mut model = parse_model(
        Some(Path::new(
            "benches/scale/corpus/pilot/random_magma_n4_k1_definable_eq_i1.model",
        )),
        true,
    )
    .unwrap();
    model.relations.retain(|s, _| !s.starts_with('T'));
    for &a in &[0i64, 1, 2, 3] {
        let k = qf_isotype_equality_key(&model, &[a]);
        // decode length
        let n = u32::from_le_bytes(k[0..4].try_into().unwrap());
        println!("a={a} history_len={n}");
    }
}

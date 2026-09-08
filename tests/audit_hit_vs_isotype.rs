
use opendefalgsplitting::engines::tuple_model_hash::TupleModelHash;
use opendefalgsplitting::hit::qf_isotype_equality_key;
use opendefalgsplitting::parser::parse_model;
use std::collections::HashMap;
use std::path::Path;

#[test]
fn audit_n4_keys() {
    let mut model = parse_model(
        Some(Path::new("benches/scale/corpus/pilot/random_magma_n4_k1_definable_eq_i1.model")),
        true,
    )
    .unwrap();
    model.relations.retain(|s, _| !s.starts_with('T'));
    let mut tmh: HashMap<Vec<u8>, Vec<i64>> = HashMap::new();
    let mut hit: HashMap<Vec<u8>, Vec<i64>> = HashMap::new();
    for &a in &model.universe.clone() {
        let h = TupleModelHash::compute(&model, &[a]);
        // fingerprint via eq classes: use Debug of relations + type sets
        let key = format!("{:?}", h.type_index_sets_for_test());
        tmh.entry(key.into_bytes()).or_default().push(a);
        let k = qf_isotype_equality_key(&model, &[a]);
        hit.entry(k).or_default().push(a);
    }
    println!("TMH classes: {:?}", tmh.values().collect::<Vec<_>>());
    println!("HIT-key classes: {:?}", hit.values().collect::<Vec<_>>());
    assert!(tmh.len() >= 1);
}

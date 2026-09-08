use opendefalgsplitting::engines::tuple_model_hash::TupleModelHash;
use opendefalgsplitting::hit::qf_isotype_equality_key;
use opendefalgsplitting::parser::parse_model;
use std::path::Path;

#[test]
fn dump_histories() {
    let mut model = parse_model(
        Some(Path::new(
            "benches/scale/corpus/pilot/random_magma_n4_k1_definable_eq_i1.model",
        )),
        true,
    )
    .unwrap();
    model.relations.retain(|s, _| !s.starts_with('T'));
    for &a in &model.universe.clone() {
        let h = TupleModelHash::compute(&model, &[a]);
        let k = qf_isotype_equality_key(&model, &[a]);
        println!(
            "a={a} tmh_flat={:?} tmh_sets={:?} hit_key_len={} hit_key={:?}",
            h.flat_h,
            h.type_index_sets_for_test(),
            k.len(),
            k
        );
    }
}

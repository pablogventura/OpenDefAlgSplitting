use opendefalgsplitting::engines::tuple_model_hash::TupleModelHash;
use opendefalgsplitting::hit::qf_isotype_equality_key;
use opendefalgsplitting::parser::parse_model;
use std::path::Path;

#[test]
fn compare_11_13() {
    let mut model = parse_model(
        Some(Path::new(
            "benches/scale/corpus/pilot/random_magma_n16_k1_random_half_i1.model",
        )),
        true,
    )
    .unwrap();
    // target membership
    let t = model
        .relations
        .values()
        .find(|r| r.sym.starts_with('T'))
        .cloned()
        .unwrap();
    println!("11 in T? {} 13 in T? {}", t.contains(&[11]), t.contains(&[13]));
    model.relations.retain(|s, _| !s.starts_with('T'));
    let h11 = TupleModelHash::compute(&model, &[11]);
    let h13 = TupleModelHash::compute(&model, &[13]);
    println!("TMH equal? {}", h11 == h13);
    println!("TMH11 flat {:?} sets {:?}", h11.flat_h, h11.type_index_sets_for_test());
    println!("TMH13 flat {:?} sets {:?}", h13.flat_h, h13.type_index_sets_for_test());
    let k11 = qf_isotype_equality_key(&model, &[11]);
    let k13 = qf_isotype_equality_key(&model, &[13]);
    println!("HIT key equal? {} len {} {}", k11 == k13, k11.len(), k13.len());
}

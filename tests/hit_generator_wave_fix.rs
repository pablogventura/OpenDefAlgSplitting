//! Regression: HIT generator must continue waves after promoting `nuevos`
//! (false NOT DEFINABLE on magmas where IsoType merge is yes).

use opendefalgsplitting::hit::{is_open_def, HitConfig};
use opendefalgsplitting::parser::parse_model;
use std::path::Path;

#[test]
fn hit_agrees_merge_on_former_false_negative() {
    let path = Path::new(
        "benches/scale/corpus/pilot/random_magma_n4_k1_definable_eq_i1.model",
    );
    let mut model = parse_model(Some(path), true).expect("parse");
    let targets: Vec<_> = model
        .relations
        .keys()
        .filter(|s| s.starts_with('T'))
        .cloned()
        .collect();
    let mut tgs = Vec::new();
    for s in targets {
        tgs.push(model.relations.remove(&s).expect("T"));
    }
    assert!(
        is_open_def(&model, tgs, HitConfig::default()).is_ok(),
        "HIT must find singleton definable when IsoTypes distinguish elements"
    );
}

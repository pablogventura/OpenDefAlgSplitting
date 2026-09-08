//! L2 golden: TupleModelHash vs OpenDefAlgMerging dump for suma4.

use opendefalgsplitting::engines::tuple_model_hash::TupleModelHash;
use opendefalgsplitting::parser::parse_model;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct Golden {
    rows: Vec<GoldenRow>,
}

#[derive(Debug, Deserialize)]
struct GoldenRow {
    tuple: Vec<i64>,
    flat_h: Vec<i64>,
    type_sets: Vec<Vec<usize>>,
    #[serde(rename = "R")]
    r: Vec<Vec<Vec<usize>>>,
}

#[test]
fn suma4_isotype_matches_oracle_json() {
    let golden_path = Path::new("benches/parity/suma4_isotype_oracle.json");
    let raw = std::fs::read_to_string(golden_path).expect("golden json");
    let golden: Golden = serde_json::from_str(&raw).expect("parse golden");

    let mut model = parse_model(Some(Path::new("model_examples/suma4.model")), true).expect("parse");
    model.relations.retain(|s, _| !s.starts_with('T'));

    for row in &golden.rows {
        let h = TupleModelHash::compute(&model, &row.tuple);
        assert_eq!(
            h.flat_h, row.flat_h,
            "flat_h mismatch for {:?}",
            row.tuple
        );
        let rust_sets: BTreeSet<BTreeSet<usize>> = h
            .type_index_sets_for_test()
            .iter()
            .cloned()
            .collect();
        let py_sets: BTreeSet<BTreeSet<usize>> = row
            .type_sets
            .iter()
            .map(|v| v.iter().copied().collect())
            .collect();
        assert_eq!(rust_sets, py_sets, "type sets for {:?}", row.tuple);

        let rust_r: Vec<BTreeSet<Vec<usize>>> = h.relations_fp_for_test().to_vec();
        let py_r: Vec<BTreeSet<Vec<usize>>> = row
            .r
            .iter()
            .map(|bucket| bucket.iter().cloned().collect())
            .collect();
        assert_eq!(rust_r, py_r, "R for {:?}", row.tuple);
    }
}

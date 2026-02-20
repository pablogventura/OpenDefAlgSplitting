use std::collections::HashMap;

use super::relops::{Operation, Relation};

#[derive(Debug, Clone)]
pub struct Model {
    pub universe: Vec<i64>,
    pub relations: HashMap<String, Relation>,
    pub operations: HashMap<String, Operation>,
}

impl Model {
    pub fn new(
        universe: Vec<i64>,
        relations: HashMap<String, Relation>,
        operations: HashMap<String, Operation>,
    ) -> Self {
        let mut u = universe;
        u.sort_unstable();
        Self {
            universe: u,
            relations,
            operations,
        }
    }
}

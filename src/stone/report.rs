use serde::Serialize;

use crate::stone::preserve::StoneOp;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FilterReport {
    pub raw_op_count: usize,
    pub filtered_op_count: usize,
    pub operations: Vec<String>,
}

impl FilterReport {
    pub fn from_ops(raw_op_count: usize, filtered_op_count: usize, operations: Vec<StoneOp>) -> Self {
        Self {
            raw_op_count,
            filtered_op_count,
            operations: operations.iter().map(op_label).collect(),
        }
    }
}

pub fn op_label(op: &StoneOp) -> String {
    match op {
        StoneOp::Discriminator => "discriminator".into(),
        StoneOp::FBar { bar, b } => format!("f_{:?}_{}", bar, b),
        StoneOp::GD { domain } => format!("g_{:?}", domain),
    }
}

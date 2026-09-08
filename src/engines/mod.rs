//! Partition engines for DefLab fragments (parity with fopy.engines).

pub mod partition;
pub mod positive;
pub mod types;
pub mod morphisms;
pub mod morph;
pub mod tuple_model_hash;
pub mod megahit;
pub mod guarded;
pub mod horn;
pub mod dispatch;

pub use dispatch::{check_engine, EngineKind, EngineOutcome, FragmentKind};

//! Stone realizability (StoneGral.pdf): Alg. 1-2 over finite universes.

pub mod discriminator;
pub mod filter;
pub mod preserve;
pub mod report;
pub mod spec;

pub use discriminator::discriminator;
pub use filter::filtering_functions;
pub use preserve::{not_preserves_sub_sq, preserves_op};
pub use report::FilterReport;
pub use spec::{PartialIso, StoneSpec};

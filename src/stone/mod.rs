//! Stone realizability (StoneGral.pdf): Alg. 1-2 over finite universes.
//!
//! Preferred path: gap-driven Baker-Pixley synthesis (`gap_filter`).
//! Legacy / certificate path: Thm 3.2 index enumeration + Alg. 2 (`filter`).
//!
//! Name map (do not cross with Lean):
//! - Rust `filtering_functions` = Lean `gapDrivenFiltering` (preferred; golden fixtures).
//! - Rust `filtering_functions_thm32` = Lean `filteringFunctions` (Thm 3.2 certificate).

pub mod discriminator;
pub mod filter;
pub mod gap_filter;
pub mod preserve;
pub mod report;
pub mod spec;

pub use discriminator::discriminator;
pub use filter::filtering_functions as filtering_functions_thm32;
pub use gap_filter::gap_driven_filtering as filtering_functions;
pub use preserve::{not_preserves_sub_sq, preserves_op};
pub use report::FilterReport;
pub use spec::{PartialIso, StoneSpec};

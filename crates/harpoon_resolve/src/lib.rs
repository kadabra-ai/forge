pub mod resolve;
pub mod scope;

pub use resolve::{
    detect_specialization_cycles, emit_unresolved_errors, finalize_resolution, resolve_pass,
};

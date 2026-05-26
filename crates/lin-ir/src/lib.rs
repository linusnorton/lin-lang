pub mod ir;
pub mod lower;
pub mod liveness;
pub mod rc_elide;

pub use ir::*;
pub use lower::lower_module;

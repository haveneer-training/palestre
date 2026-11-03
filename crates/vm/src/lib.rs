//! Machine virtuelle `PBC v1` : exécute un tour d'un programme chargé (annexe B).
#![forbid(unsafe_code)]
#![deny(clippy::indexing_slicing, clippy::arithmetic_side_effects)]

mod machine;
mod report;

pub use machine::{Machine, Step};
pub use report::{ActKind, ActRecord, Action, Report};

/// Pile de données : 64 emplacements (B.1).
pub const STACK_SIZE: usize = 64;
/// Pile d'appels : 16 adresses de retour (B.1).
pub const CALL_DEPTH: usize = 16;
/// Mémoire d'un agent : 256 cellules, indices `0..=255` (B.1).
pub const MEM_SIZE: usize = 256;

/// La mémoire persistante d'un agent.
pub type Memory = [i64; MEM_SIZE];

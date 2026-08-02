//! The mutating half of `file_ops`: copy, move, create ([`copy_move`]), and
//! the line-range editor ([`edit`]). Re-exported here so callers keep using
//! `file_ops::{copy_file_or_directory, edit_file, ...}` regardless of the
//! split.

mod copy_move;
mod edit;

pub use copy_move::{copy_file_or_directory, create_directory, move_file_or_directory};
pub use edit::{edit_file, write_file, EditChunk};

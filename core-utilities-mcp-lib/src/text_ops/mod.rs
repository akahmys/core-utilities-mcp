//! Line-windowed file reading and structured-data querying: paging through
//! large files with line-number annotations ([`read`]), filtering/sorting
//! CSV-like matrices ([`matrix`]), and querying JSON/TOML/YAML by path
//! ([`query`]).

mod matrix;
mod query;
mod read;

pub use matrix::filter_and_sort_matrix_columns;
pub use query::query_data_by_path;
pub use read::{read_file, ReadFileResult};

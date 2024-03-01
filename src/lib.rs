#![warn(missing_debug_implementations, missing_copy_implementations)]
#![deny(trivial_casts, trivial_numeric_casts)]

pub mod api;
pub mod domain;

pub mod application;
pub mod infrastructure;

use serde::Deserialize;
use time::OffsetDateTime;

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct Statement {
    pub balance: i32,
    pub time_of_statement: OffsetDateTime,
    pub credit_limit: u32,
}

pub type AnyResult<T> = eyre::Result<T>;

#![deny(
    missing_copy_implementations,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts
)]

pub mod api;
pub mod domain;

pub mod application;
pub mod infrastructure;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub struct Statement {
    pub total: i32,
    pub data_extrato: OffsetDateTime,
    pub limite: u32,
}

pub type AnyResult<T> = eyre::Result<T>;

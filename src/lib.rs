#![warn(missing_copy_implementations, missing_debug_implementations)]
#![feature(slice_split_once)]

pub mod server_impl;

pub mod api;
pub mod domain;

pub mod application;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Deserialize, Serialize)]
struct Statement {
    pub total: i32,
    pub data_extrato: OffsetDateTime,
    pub limite: u32,
}

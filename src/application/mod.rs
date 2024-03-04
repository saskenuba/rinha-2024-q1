pub mod adapters;
pub mod cache;
pub mod repositories;

use crate::application::repositories::HeedDB;
use heed::Env;

#[derive(Clone)]
pub struct ServerData {
    pub re_conn: redis::aio::ConnectionManager,
    pub lmdb_conn: (Env, HeedDB),
}

pub mod adapters;
pub mod cache;
pub mod repositories;

use deadpool_postgres::Pool;

#[derive(Clone)]
pub struct ServerData {
    pub re_conn: redis::aio::ConnectionManager,
    pub pg_pool: Pool,
}

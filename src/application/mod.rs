pub mod adapters;

use deadpool_postgres::Pool;
use std::sync::Arc;

#[derive(Copy, Clone, Debug)]
pub struct AccountService {}

impl AccountService {
    fn get_balance() {

        // get user from database
        // and store transactions on redis so we can get to the balance faster
    }
}

#[derive(Clone)]
pub struct ServerData {
    pub re_conn: redis::aio::ConnectionManager,
    pub pg_pool: Pool,
}

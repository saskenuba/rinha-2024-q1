use redis::Client;
use std::sync::Arc;

#[derive(Copy, Clone, Debug)]
pub struct AccountService {}

impl AccountService {
    fn get_balance() {

        // get user from database
        // and store transactions on redis so we can get to the balance faster
    }
}

#[derive(Debug, Clone)]
pub struct ServerData {
    pub redis: Arc<Client>,
}

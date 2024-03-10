use crate::infrastructure::TransactionIPCRepository;

pub mod adapters;
pub mod repositories;

#[derive(Clone)]
pub struct ServerData {
    pub ipc_repo: TransactionIPCRepository,
}

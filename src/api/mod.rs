use crate::application::adapters::{StatementDTO, TransactionDTO, TransactionResponseDTO};
use crate::application::ServerData;
use crate::domain::account::Account;
use crate::domain::transaction::Transaction;
use crate::infrastructure::server_impl::request::Request;
use crate::infrastructure::server_impl::response::{JsonResponse, Response, StatusCode};
use crate::infrastructure::server_impl::server::Method;
use crate::infrastructure::TransactionIPCRepository;
use crate::AnyResult;
use eyre::bail;
use fnv::FnvHashMap;
use std::sync::{Arc, Mutex};

pub mod input_types;

pub async fn statement_route(
    server_data: &ServerData,
    req: Request<'_>,
    client_id: i32,
) -> AnyResult<Response> {
    if req.method != Method::GET {
        bail!("Only GET available.")
    }

    let service = BankAccountService {
        ipc_repo: server_data.ipc_repo.clone(),
    };

    let res = service
        .query(AccountQueries::Statement { account: client_id })
        .await;

    let a = JsonResponse::from::<StatementDTO>(StatementDTO::from_other(res));
    Ok(a.0)
}

pub async fn transaction_route(
    server_data: &ServerData,
    req: Request<'_>,
    client_id: i32,
) -> AnyResult<Response> {
    if req.method != Method::POST {
        eprintln!("body needed.");
        bail!("Only POST available.");
    }

    let body = req
        .body
        .ok_or_else(|| Response::from_status_code(StatusCode::NotFound, "no-body".to_string()))
        .unwrap();

    let mapping = serde_json::from_slice::<TransactionDTO>(body)
        .map_err(|_| ())
        .and_then(|c| c.try_into());

    let transaction: Transaction = match mapping {
        Ok(trans) => trans,
        Err(_) => {
            return Ok(Response::from_status_code(
                StatusCode::NotFound,
                "error-mapping".to_string(),
            ));
        }
    };

    let command = AccountCommands::HandleMoney {
        account: client_id,
        transaction,
    };

    let bank_service = BankAccountService {
        ipc_repo: server_data.ipc_repo.clone(),
    };

    let Ok(acc) = bank_service.handler(command).await else {
        return Ok(Response::from_status_code(
            StatusCode::UnprocessableEntity,
            None,
        ));
    };

    let response = JsonResponse::from::<TransactionResponseDTO>(TransactionResponseDTO::from(acc));
    Ok(response.0)
}

pub type AccountMapStorage = Arc<FnvHashMap<i32, (u32, Mutex<Account>)>>;

struct BankAccountService {
    ipc_repo: TransactionIPCRepository,
}

#[derive(Debug)]
enum AccountCommands {
    HandleMoney {
        account: i32,
        transaction: Transaction,
    },
}

#[derive(Debug)]
enum AccountQueries {
    Statement { account: i32 },
}

impl BankAccountService {
    async fn query(&self, command: AccountQueries) -> (Account, impl Iterator<Item = Transaction>) {
        match command {
            AccountQueries::Statement { account: user } => unsafe {
                let (acc, trans) = self.ipc_repo.get_acc_and_transactions(user);
                (acc, trans.into_iter())
            },
        }
    }

    async fn handler(&self, command: AccountCommands) -> AnyResult<Account> {
        match command {
            AccountCommands::HandleMoney {
                account: user,
                transaction,
            } => {
                let acc = unsafe {
                    // guard resource
                    let guard = self.ipc_repo.lock_account_resources(user).await;

                    let (acc, _) = self.ipc_repo.get_acc_and_transactions(user);
                    let acc = acc
                        .add_transaction(&transaction)
                        .or_else(|_| bail!("no credits bro"))?;
                    self.ipc_repo.add_transaction(&acc, &transaction);

                    // remove guard
                    self.ipc_repo.unlock(guard, user);

                    acc
                };

                Ok(acc)
            }
        }
    }
}

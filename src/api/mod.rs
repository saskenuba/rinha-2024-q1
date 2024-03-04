use crate::application::adapters::{StatementDTO, TransactionDTO, TransactionResponseDTO};
use crate::application::repositories::{HeedDB, TransactionLMDBRepository};
use crate::application::ServerData;
use crate::domain::account::Account;
use crate::domain::transaction::Transaction;
use crate::infrastructure::redis_lock::RedisLock;
use crate::infrastructure::server_impl::request::Request;
use crate::infrastructure::server_impl::response::{JsonResponse, Response, StatusCode};
use crate::infrastructure::server_impl::server::Method;
use crate::AnyResult;
use eyre::bail;
use fnv::FnvHashMap;
use heed::Env;
use redis::aio::ConnectionManager;
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
        re_conn: server_data.re_conn.clone(),
        lmdb_conn: server_data.lmdb_conn.clone(),
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
        re_conn: server_data.re_conn.clone(),
        lmdb_conn: server_data.lmdb_conn.clone(),
    };
    let acc = bank_service.handler(command).await;
    eprintln!("{:?}", acc);

    let Ok(acc) = acc else {
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
    re_conn: ConnectionManager,
    lmdb_conn: (Env, HeedDB),
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
            AccountQueries::Statement { account: user } => {
                let trans_repo = TransactionLMDBRepository {
                    db: self.lmdb_conn.clone(),
                };

                let acc = trans_repo.get_account(user).unwrap();
                let trans = trans_repo.get_last_10(user);
                (acc, trans.into_iter())
            }
        }
    }
    async fn handler(&self, command: AccountCommands) -> AnyResult<Account> {
        match command {
            AccountCommands::HandleMoney {
                account: user,
                transaction,
            } => {
                let trans_repo = TransactionLMDBRepository {
                    db: self.lmdb_conn.clone(),
                };

                let redis_lock = RedisLock::new(self.re_conn.clone(), user, 300);
                let acc = {
                    let guard = redis_lock.acquire().await.unwrap();

                    let acc = trans_repo.get_account(user)?;
                    let acc = acc
                        .add_transaction(&transaction)
                        .or_else(|_| bail!("no credits bro"))?;
                    trans_repo.save_transaction(&acc, transaction);

                    guard.release().await;
                    acc
                };

                Ok(acc)
            }
        }
    }
}

use crate::application::adapters::{StatementDTO, TransactionDTO};
use crate::application::cache::AccountCache;
use crate::application::repositories::TransactionRepository;
use crate::application::ServerData;
use crate::domain::account::Account;
use crate::domain::transaction::Transaction;
use crate::infrastructure::server_impl::request::Request;
use crate::infrastructure::server_impl::response::{JsonResponse, Response, StatusCode};
use crate::infrastructure::server_impl::server::Method;
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
    // let body = req.body.unwrap();

    if req.method != Method::GET {
        bail!("Only GET available.")
    }

    let service = BankAccountService {
        re_conn: server_data.re_conn.clone(),
        pg_conn: server_data.pg_pool.clone(),
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

    // read model
    // - read last 10 transactions and balance from redis

    // write model
    // * debit *
    // take lock    -> persist to database
    //              -> insert transaction and get balance
    //              -> insert transaction and balance on redis
    //              -> remove lock

    // * credit *
    // take lock    -> persist to database
    //              -> insert transaction and get balance
    //              -> insert transaction and balance on redis
    //              -> remove lock

    let command = AccountCommands::HandleMoney {
        account: client_id,
        transaction,
    };

    let bank_service = BankAccountService {
        re_conn: server_data.re_conn.clone(),
        pg_conn: server_data.pg_pool.clone(),
    };
    let res = bank_service.handler(command).await;

    // if res.is_err() {
    //     return Ok(Response::from_status_code(
    //         StatusCode::UnprocessableEntity,
    //         None,
    //     ));
    // }

    let a =
        JsonResponse::from::<TransactionDTO>(TransactionDTO::from(Transaction::generate(1, None)));
    Ok(a.0)
}

pub type AccountMapStorage = Arc<FnvHashMap<i32, (u32, Mutex<Account>)>>;

struct BankAccountService {
    re_conn: redis::aio::ConnectionManager,
    pg_conn: deadpool_postgres::Pool,
    // storage: AccountMapStorage,
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
                let trans_cache = AccountCache {
                    re_conn: self.re_conn.clone(),
                };

                let res = trans_cache.get_account(user, true).await;
                (res.0, res.1.unwrap())
            }
        }
    }
    async fn handler(&self, command: AccountCommands) -> AnyResult<()> {
        match command {
            AccountCommands::HandleMoney {
                account: user,
                transaction,
            } => {
                let trans_repo = TransactionRepository {
                    conn: self.pg_conn.clone(),
                };

                let trans_cache = AccountCache {
                    re_conn: self.re_conn.clone(),
                };

                // let redis_lock = RedisLock::new(self.re_conn.clone(), user, 100);

                let (acc, _) = trans_cache.get_account(user, false).await;
                let acc = acc
                    .add_transaction(&transaction)
                    .or_else(|_| bail!("no credits bro"))?;

                {
                    // let guard = redis_lock.acquire().await.unwrap();
                    trans_repo.save_and_get_balance(user, &transaction).await?;
                    trans_cache
                        .save_account(user, &acc, Some(&transaction))
                        .await;
                    // guard.release().await;
                }

                Ok(())
            }
        }
    }
}

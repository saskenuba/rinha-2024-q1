use crate::application::adapters::{StatementDTO, TransactionDTO};
use crate::application::ServerData;
use crate::domain::transaction::Transaction;
use crate::infrastructure::redis_lock::RedisLock;
use crate::infrastructure::server_impl::request::Request;
use crate::infrastructure::server_impl::response::{JsonResponse, Response, StatusCode};
use crate::infrastructure::server_impl::server::Method;
use crate::{AnyResult, Statement};
use compact_str::CompactString;
use eyre::bail;
use redis::streams::StreamRangeReply;
use redis::{AsyncCommands, Client, Value};
use serde::{Deserialize, Serialize};
use std::mem;
use std::sync::Arc;
use time::OffsetDateTime;

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

    let my = Statement {
        balance: 0,
        time_of_statement: OffsetDateTime::now_utc(),
        credit_limit: 0,
    };

    let a = JsonResponse::from::<StatementDTO>(my.into());
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
        .map_err(|e| ())
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

    let command = AccountCommands::WithdrawMoney {
        account: client_id,
        transaction: &transaction,
    };

    let res = BankAccountService {
        re_conn: server_data.re_conn.clone(),
        pg_conn: server_data.pg_pool.clone(),
    }
    .handler(command)
    .await;

    let a = JsonResponse::from::<TransactionDTO>(transaction.into());
    Ok(a.0)
}

struct BankAccountService {
    re_conn: redis::aio::ConnectionManager,
    pg_conn: deadpool_postgres::Pool,
}

enum AccountCommands<'a> {
    DepositMoney {
        user: i32,
        amount: &'a Transaction,
    },
    WithdrawMoney {
        account: i32,
        transaction: &'a Transaction,
    },
}

enum AccountQueries {
    Statement,
}

impl BankAccountService {
    async fn handler(&self, command: AccountCommands<'_>) {
        match command {
            AccountCommands::DepositMoney { .. } => unimplemented!(),
            AccountCommands::WithdrawMoney {
                account: user,
                transaction,
            } => {
                let trans_repo = TransactionRepository {
                    conn: self.pg_conn.clone(),
                };

                let trans_cache = TransactionCache {
                    re_conn: self.re_conn.clone(),
                };

                let redis_lock = RedisLock::new(self.re_conn.clone(), user, 100);
                {
                    let guard = redis_lock.acquire().await.unwrap();
                    trans_repo.save_transaction(user, transaction).await;
                    trans_cache.append(user, transaction).await;
                    guard.release().await;
                }
            }
        }
    }
}

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

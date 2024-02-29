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

#[derive(Debug, Serialize, Deserialize)]
struct Statementredis {
    balance: i32,
    transactions: Vec<Transaction>,
}

struct TransactionRepository {
    conn: deadpool_postgres::Pool,
}

impl TransactionRepository {
    async fn save_transaction(&self, user_id: i32, transaction: &Transaction) {
        let conn = self.conn.get().await.unwrap();

        let query = r#"
          WITH insertion
           AS (INSERT INTO transaction (amount, description, account_id) VALUES ($1, $2, $3) 
               RETURNING account_id, amount )
  SELECT rb.account_id, running_balance + i.amount
  FROM running_balance rb
       INNER JOIN insertion i ON rb.account_id = i.account_id;"#;
        let stmt = conn.prepare_cached(query).await.unwrap();

        let desc = transaction.descricao.0.as_str();
        let amount = transaction.valor;

        let res = conn
            .execute(&stmt, &[&i32::from(amount), &desc, &user_id])
            .await
            .unwrap();
    }
}

struct TransactionCache {
    re_conn: redis::aio::ConnectionManager,
}

impl TransactionCache {
    const TRANSACTIONS_KEY: &'static str = "transactions";

    fn key_fn(user_id: i32) -> CompactString {
        let key = Self::TRANSACTIONS_KEY;
        compact_str::format_compact!("{key}:{user_id}")
    }

    pub async fn get_latest_n(&self, n: i32) -> Vec<Transaction> {
        let key = Self::key_fn(1);

        let stream_result: StreamRangeReply = self
            .re_conn
            .clone()
            .xrevrange_count(key.as_str(), "+", "-", n)
            .await
            .unwrap();

        stream_result
            .ids
            .into_iter()
            .flat_map(|v| v.map.into_iter())
            .filter_map(|(_, val)| {
                if let Value::Data(data) = val {
                    let transaction = bitcode::deserialize::<Transaction>(&data).unwrap();
                    Some(transaction)
                } else {
                    None
                }
            })
            .collect::<Vec<Transaction>>()
    }

    pub async fn append(&self, user_id: i32, transaction: &Transaction) {
        let serialized = bitcode::serialize(&transaction).unwrap();
        let key = Self::key_fn(user_id);

        self.re_conn
            .clone()
            .xadd(key.as_str(), "*", &[(user_id, serialized)])
            .await
            .unwrap()
    }
}

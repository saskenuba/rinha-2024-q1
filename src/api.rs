use crate::application::ServerData;
use crate::infrastructure::redis_lock::RedisLock;
use crate::infrastructure::server_impl::request::Request;
use crate::infrastructure::server_impl::response::{JsonResponse, Response};
use crate::infrastructure::server_impl::server::Method;
use crate::{AnyResult, Statement};
use eyre::bail;
use redis::Client;
use std::sync::Arc;
use time::OffsetDateTime;

pub fn statement_route(
    server_data: &ServerData,
    req: Request,
    client_id: i32,
) -> AnyResult<Response> {
    // let body = req.body.unwrap();

    if req.method != Method::GET {
        bail!("Only GET available.")
    }

    let my = Statement {
        total: 0,
        data_extrato: OffsetDateTime::now_utc(),
        limite: 0,
    };

    let a = JsonResponse::from::<Statement>(my);
    Ok(a.0)
}
pub fn transaction_route(
    server_data: &ServerData,
    req: Request,
    client_id: i32,
) -> AnyResult<Response> {
    if req.method != Method::POST {
        bail!("Only POST available.")
    }

    // let command = BankAccountCommand::Debit { amount: 1 };
    // bank_query.execute(connection, command)

    // Debit Transaction
    //
    // need locks to ensure balance isn't screwed up
    // let lock = queue.lock.await;
    // pg.persist(transaction_debit);

    // Credit Transaction
    //
    // don't need locks to insert one more into a list
    // pg.persist(transaction_debit);

    todo!()
}

struct BankAccountService {
    redis: Arc<Client>,
    // users_lock: FnvHashSet<i32>
}

enum AccountCommands {
    DepositMoney { amount: i64 },
    WithdrawMoney { user: i32, amount: i64 },
}

enum AccountQueries {
    Statement,
}

impl BankAccountService {
    async fn handler(&self, command: AccountCommands) {
        match command {
            AccountCommands::DepositMoney { .. } => {}
            AccountCommands::WithdrawMoney { user, amount } => {
                let lock = RedisLock::new(self.redis.clone(), user, 300);
                let _ = lock.acquire();
            }
        }
    }
}

use crate::domain::account::Account;
use crate::domain::transaction::Transaction;
use compact_str::CompactString;
use redis::streams::StreamRangeReply;
use redis::{AsyncCommands, Value};

pub struct AccountCache {
    pub re_conn: redis::aio::ConnectionManager,
}

impl AccountCache {
    const ACCOUNT_KEY: &'static str = "account";
    const TRANSACTIONS_KEY: &'static str = "transactions";

    fn key_trans_fn(user_id: i32) -> CompactString {
        let key = Self::TRANSACTIONS_KEY;
        compact_str::format_compact!("{key}:{user_id}")
    }
    fn key_acc_fn(user_id: i32) -> CompactString {
        let key = Self::ACCOUNT_KEY;
        compact_str::format_compact!("{key}:{user_id}")
    }

    pub async fn get_account(
        &self,
        user_id: i32,
        with_transactions: bool,
    ) -> (Account, Option<impl Iterator<Item = Transaction>>) {
        let trans_key = Self::key_trans_fn(user_id);
        let acc_key = Self::key_acc_fn(user_id);

        if with_transactions {
            let mut pipe = redis::pipe();
            let mut conn = self.re_conn.clone();
            let res: (StreamRangeReply, Vec<u8>) = pipe
                .xrevrange_count(trans_key.as_str(), "+", "-", 10)
                .get(acc_key.as_str())
                .query_async(&mut conn)
                .await
                .unwrap();

            let acc = bitcode::deserialize::<Account>(&res.1).unwrap();
            let transactions = res
                .0
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
                });

            (acc, Some(transactions))
        } else {
            let acc = self
                .re_conn
                .clone()
                .get(acc_key.as_str())
                .await
                .ok()
                .and_then(|bytes: Vec<u8>| bitcode::deserialize::<Account>(&bytes).ok())
                .unwrap();
            (acc, None)
        }
    }

    pub async fn save_account(
        &self,
        user_id: i32,
        acc: &Account,
        with_transaction: Option<&Transaction>,
    ) {
        let acc_serialized = bitcode::serialize(&acc).unwrap();
        let trans_key = Self::key_trans_fn(user_id);
        let acc_key = Self::key_acc_fn(user_id);

        let mut like = self.re_conn.clone();
        let mut pipeline = redis::pipe();
        pipeline.set(acc_key.as_str(), acc_serialized);

        if let Some(trans) = with_transaction {
            let trans_serialized = bitcode::serialize(&trans).unwrap();
            pipeline
                .xadd(trans_key.as_str(), "*", &[(user_id, trans_serialized)])
                .ignore();
        }

        pipeline.query_async(&mut like).await.unwrap()
    }
}

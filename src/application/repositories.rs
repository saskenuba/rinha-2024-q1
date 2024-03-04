use crate::domain::account::Account;
use crate::domain::transaction::Transaction;
use crate::AnyResult;
use eyre::OptionExt;
use heed::byteorder::BigEndian;
use std::cmp::{min, Reverse};
use std::ops::Range;
use time::OffsetDateTime;

pub type HeedDB = heed::Database<heed::types::I32<BigEndian>, heed::types::Bytes>;

#[derive(Debug)]
#[allow(non_snake_case)]
pub struct TransactionLMDBRepository {
    pub db: (heed::Env, HeedDB),
}

pub trait HasLMDBKeyRange {
    fn key_range(user_id: i32) -> Range<i32>;
    fn key_max(user_id: i32) -> i32;
}
impl HasLMDBKeyRange for Transaction {
    fn key_range(user_id: i32) -> Range<i32> {
        let t_size = 10;
        let start = user_id + 10;
        let range = start..start + t_size;
        range
    }
    fn key_max(user_id: i32) -> i32 {
        Self::key_range(user_id).last().unwrap() + 1
    }
}

pub trait HasLMDBKey {
    fn key(&self, user_id: Option<i32>) -> i32;
}
impl HasLMDBKey for Account {
    fn key(&self, user_id: Option<i32>) -> i32 {
        i32::MAX - user_id.unwrap_or(self.id)
    }
}

impl TransactionLMDBRepository {
    pub fn get_account(&self, user_id: i32) -> AnyResult<Account> {
        let db = self.db.1;
        let env = &self.db.0;
        let rtx = env.read_txn().unwrap();

        let id = i32::MAX - user_id;
        db.get(&rtx, &id)?
            .and_then(|bytes| bitcode::deserialize::<Account>(bytes).ok())
            .ok_or_eyre("Account not found.")
    }

    pub fn save_account(&self, account: &Account) {
        let db = self.db.1;
        let env = &self.db.0;
        let mut rwtx = env.write_txn().unwrap();

        let key = account.key(None);
        let acc_serialized = bitcode::serialize(account).expect("Safe");
        db.put(&mut rwtx, &key, &acc_serialized).unwrap();
        rwtx.commit().unwrap();
    }

    /// Returns the last 10 transactions ordered from newest to oldest.
    pub fn get_last_10(&self, user_id: i32) -> Vec<Transaction> {
        let db = self.db.1;
        let env = &self.db.0;
        let rtx = env.read_txn().unwrap();

        let t_idxs = Transaction::key_range(user_id);
        let t_coll = db.rev_range(&rtx, &t_idxs).unwrap().flatten();
        let mut buf = Vec::with_capacity(10);
        for (_, bytes) in t_coll {
            let transaction = bitcode::deserialize::<Transaction>(bytes).expect("Safe to unwrap");
            buf.push(transaction);
        }
        buf.sort_unstable_by_key(|t| Reverse(t.realizada_em));
        buf
    }

    pub fn save_transaction(&self, account: &Account, new_transaction: Transaction) {
        let db = self.db.1;
        let env = &self.db.0;

        let mut t_range = Transaction::key_range(account.id);
        let t_range_max = Transaction::key_max(account.id) as usize;
        let transaction = bitcode::serialize::<Transaction>(&new_transaction).unwrap();

        // check if full index is really full (10 entries)
        // if positive, remove the oldest and just insert.. we will sort them on read
        // if negative, insert at the next available index
        let mut rwtx = env.write_txn().unwrap();
        let db_range = db.range(&rwtx, &t_range).unwrap().collect::<Vec<_>>();

        let mut oldest_offset = OffsetDateTime::now_utc();
        let mut oldest_idx = 0;
        let db_keys_len = db_range.len();

        if db_keys_len < 10 {
            let insertion_idx = (t_range_max - 10 + db_keys_len) as i32;
            db.put(&mut rwtx, &insertion_idx, &transaction).unwrap();
        } else {
            let mut db_range = db_range.into_iter();

            while let (Some(t_range_idx), Some(t_found)) = (t_range.next(), db_range.next()) {
                let (db_idx, db_t_data) = t_found.unwrap();
                let t = bitcode::deserialize::<Transaction>(db_t_data).unwrap();

                if min(oldest_offset, t.realizada_em) != oldest_offset {
                    oldest_offset = t.realizada_em;
                    oldest_idx = t_range_idx
                }
            }
            let insertion_idx = oldest_idx;
            db.put(&mut rwtx, &insertion_idx, &transaction).unwrap();
        }

        let acc_serialized = bitcode::serialize::<Account>(account).unwrap();
        db.put(&mut rwtx, &account.key(None), &acc_serialized)
            .unwrap();
        rwtx.commit().unwrap();
    }
}

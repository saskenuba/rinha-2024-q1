use crate::domain::account::Account;
use crate::domain::transaction::Transaction;
use crate::infrastructure::adapters::TransactionDAO;
use std::mem::size_of;
use std::ops::Range;

pub trait HasOffsetRange {
    fn offset_range(user_id: i32) -> Range<usize>;
}
impl HasOffsetRange for Account {
    fn offset_range(user_id: i32) -> Range<usize> {
        let start = (user_id as usize - 1) * size_of::<Self>();
        let offset = start + size_of::<Self>();

        start..offset
    }
}

impl HasOffsetRange for Transaction {
    fn offset_range(user_id: i32) -> Range<usize> {
        let start = (user_id as usize - 1) * size_of::<Self>();
        let offset = start + size_of::<TransactionDAO>() * 10;

        start..offset
    }
}

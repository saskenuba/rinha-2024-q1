use crate::domain::errors::AccountError;
use crate::domain::transaction::Transaction;
use serde::{Deserialize, Serialize};
use zerocopy_derive::{FromBytes, IntoBytes, KnownLayout, NoCell};

#[repr(C)]
#[derive(Debug, FromBytes, IntoBytes, NoCell, KnownLayout, Deserialize, Serialize)]
pub struct Account {
    pub id: i32,
    pub balance: i32,
    pub credit_limit: u32,
}

impl Account {
    pub fn new(id: i32, credit_limit: u32) -> Self {
        Self {
            id,
            balance: 0,
            credit_limit,
        }
    }

    #[cfg(test)]
    fn generate(balance: i32, limit: u32) -> Account {
        Account {
            id: -1,
            balance,
            credit_limit: limit,
        }
    }

    /// Returns [Account] with the new transaction.
    pub fn add_transaction(mut self, transaction: &Transaction) -> Result<Self, AccountError> {
        let transaction_amount = transaction.valor;
        let new_balance = self.balance + i32::from(transaction_amount);

        if new_balance.is_negative() && new_balance.abs() > self.credit_limit as i32 {
            return Err(AccountError::InsufficientCredit);
        }

        self.balance = new_balance;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_more_funds() {
        let account = Account::generate(1_000, 10_000);
        assert_eq!(account.balance, 1_000);

        let with_new_transaction = account
            .add_transaction(&Transaction::generate(-1000, None))
            .unwrap();
        assert_eq!(with_new_transaction.balance, 0);

        let with_new_transaction = with_new_transaction
            .add_transaction(&Transaction::generate(-10_000, None))
            .unwrap();
        assert_eq!(with_new_transaction.balance, -10_000);

        let with_new_transaction =
            with_new_transaction.add_transaction(&Transaction::generate(-1, None));
        match with_new_transaction {
            Err(AccountError::InsufficientCredit) => {}
            _ => unreachable!(),
        }
    }
}

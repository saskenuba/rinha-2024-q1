use crate::domain::errors::AccountError;
use crate::domain::transaction::Transaction;

/// Aggregate
///
/// Don't store the balance on the aggregate you dummkopf!
struct Account {
    id: i32,
    transactions: Vec<Transaction>,
    credit_limit: u32,
}

impl Account {
    #[cfg(test)]
    fn generate(balance: i32, limit: u32) -> Account {
        Account {
            id: -1,
            transactions: vec![Transaction::generate(balance)],
            credit_limit: limit,
        }
    }

    /// Returns the balance based on all user transactions.
    fn balance(&self) -> i32 {
        self.transactions.iter().map(|c| i32::from(c.valor)).sum()
    }

    /// Returns [Account] with the new transaction.
    fn add_transaction(mut self, transaction: Transaction) -> Result<Self, AccountError> {
        let balance = self.balance();
        let transaction_amount = transaction.valor;
        let new_balance = balance + i32::from(transaction_amount);

        if new_balance.is_negative() && new_balance.abs() > self.credit_limit as i32 {
            return Err(AccountError::InsufficientCredit);
        }

        self.transactions.push(transaction);
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_more_funds() {
        let account = Account::generate(1_000, 10_000);
        let with_new_transaction = account
            .add_transaction(Transaction::generate(-1000))
            .unwrap();
        assert_eq!(with_new_transaction.balance(), 0);

        let with_new_transaction = with_new_transaction
            .add_transaction(Transaction::generate(-10_000))
            .unwrap();
        assert_eq!(with_new_transaction.balance(), -10_000);

        let with_new_transaction = with_new_transaction.add_transaction(Transaction::generate(-1));
        match with_new_transaction {
            Err(AccountError::InsufficientCredit) => {}
            _ => unreachable!(),
        }
    }
}

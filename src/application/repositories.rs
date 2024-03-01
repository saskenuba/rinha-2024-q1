use crate::domain::account::Account;
use crate::domain::transaction::Transaction;
use crate::AnyResult;
use eyre::bail;

#[derive(Debug)]
pub struct TransactionRepository {
    pub conn: deadpool_postgres::Pool,
}

impl TransactionRepository {
    pub async fn get_accounts(&self) -> impl IntoIterator<Item = Account> {
        let conn = self.conn.get().await.unwrap();

        let query = r#"
SELECT id
     , balance
     , credit_limit 
  FROM account;"#;
        let stmt = conn.prepare_cached(query).await.unwrap();
        let res = conn.query(&stmt, &[]).await.unwrap();

        res.into_iter().map(|r| Account {
            id: r.get(0),
            balance: r.get(1),
            credit_limit: r.get::<usize, i32>(2) as u32,
        })
    }

    /// Returns the source-of-truth balance for user_id.
    pub async fn save_and_get_balance(
        &self,
        user_id: i32,
        transaction: &Transaction,
    ) -> AnyResult<()> {
        let conn = self.conn.get().await?;

        let query = r#"
        WITH insertion
              AS (INSERT INTO transaction (amount, description, account_id) VALUES ($1, $2, $3) RETURNING account_id, amount )
   UPDATE account acc
      SET balance = balance + insertion.amount
     FROM insertion
    WHERE acc.id = insertion.account_id;"#;
        let stmt = conn.prepare_cached(query).await?;

        let desc = transaction.descricao.0.as_str();
        let amount = transaction.valor;

        conn.query_one(&stmt, &[&i32::from(amount), &desc, &user_id])
            .await
            .map(|_| ())
            .or_else(|_| bail!("problem querying postgres"))
    }
}

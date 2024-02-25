//! Domain Errors

#[derive(Debug)]
pub enum TransactionError {
    InvalidDescription,
}

#[derive(Debug)]
pub enum AccountError {
    InsufficientCredit,
}

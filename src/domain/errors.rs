//! Domain Errors

#[derive(Debug, Copy, Clone)]
pub enum TransactionError {
    InvalidDescription,
}

#[derive(Debug, Copy, Clone)]
pub enum AccountError {
    InsufficientCredit,
}

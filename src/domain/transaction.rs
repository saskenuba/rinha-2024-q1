use crate::AnyResult;
use compact_str::CompactString;
use eyre::bail;
use serde::{Deserialize, Serialize};
use std::num::NonZeroI32;
use time::OffsetDateTime;

#[derive(Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub valor: NonZeroI32,
    pub tipo: TransactionKind,
    pub descricao: TransactionDescription,
    pub realizada_em: OffsetDateTime,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum TransactionKind {
    Credit,
    Debit,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct TransactionDescription(pub CompactString);

impl TransactionDescription {
    pub fn new(description: &str) -> AnyResult<Self> {
        if description.len() > 10 && description.is_empty() {
            bail!("Invalid length");
        }

        Ok(Self(description.into()))
    }
}

impl Transaction {
    #[cfg(test)]
    pub fn generate<T>(amount: i32, description: T) -> Self
    where
        T: Into<Option<&'static str>>,
    {
        let kind = if amount.is_negative() {
            TransactionKind::Debit
        } else {
            TransactionKind::Credit
        };

        let description = TransactionDescription::new(description.into().unwrap_or("xxx"));
        Self {
            valor: NonZeroI32::new(amount).expect("Safe to unwrap"),
            tipo: kind,
            descricao: description.unwrap(),
            realizada_em: OffsetDateTime::now_utc(),
        }
    }
}

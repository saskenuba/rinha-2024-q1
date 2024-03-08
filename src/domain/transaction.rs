use std::num::NonZeroI32;

use compact_str::CompactString;
use derive_more::Deref;
use eyre::bail;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use zerocopy_derive::{FromZeros, IntoBytes, KnownLayout, NoCell};

use crate::AnyResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub valor: NonZeroI32,
    pub tipo: TransactionKind,
    pub descricao: TransactionDescription,
    pub realizada_em: OffsetDateTime,
}

impl PartialEq for Transaction {
    fn eq(&self, other: &Self) -> bool {
        self.valor == other.valor
            && self.descricao == other.descricao
            && self.tipo == other.tipo
            && self.realizada_em.unix_timestamp() == other.realizada_em.unix_timestamp()
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Self {
            valor: NonZeroI32::try_from(-1).unwrap(),
            tipo: TransactionKind::Invalid,
            descricao: TransactionDescription(CompactString::new_inline("")),
            realizada_em: OffsetDateTime::now_utc(),
        }
    }
}

#[repr(u8)]
#[derive(
    KnownLayout, NoCell, FromZeros, IntoBytes, Debug, Copy, Clone, PartialEq, Serialize, Deserialize,
)]
pub enum TransactionKind {
    /// This is very ugly hack to allow store default transactions at the database.
    Invalid = 0,

    Credit = 1,
    Debit = 2,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Deref)]
#[repr(C)]
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

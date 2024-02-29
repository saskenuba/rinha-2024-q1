use crate::domain::transaction::{Transaction, TransactionDescription, TransactionKind};
use crate::Statement;
use compact_str::{CompactString, ToCompactString};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::num::NonZeroI32;
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize)]
pub struct StatementDTO {
    #[serde(rename = "total")]
    total_balance: i32,
    #[serde(rename = "data_extrato")]
    data_extrato: CompactString,
    #[serde(rename = "limite")]
    credit_limit: u32,
}

impl From<Statement> for StatementDTO {
    fn from(value: Statement) -> Self {
        let formatted = value
            .time_of_statement
            .format(&Iso8601::DEFAULT)
            .unwrap()
            .to_compact_string();

        Self {
            total_balance: value.balance,
            data_extrato: formatted,
            credit_limit: value.credit_limit,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TransactionDTO<'a> {
    #[serde(rename = "valor")]
    pub amount: i32,
    #[serde(rename = "tipo")]
    #[serde(borrow)]
    pub kind: &'a str,
    #[serde(rename = "descricao")]
    #[serde(borrow)]
    pub description: Cow<'a, str>,
    #[serde(rename = "realizada_em")]
    #[serde(borrow)]
    pub created_on: Option<Cow<'a, str>>,
}

impl TryFrom<TransactionDTO<'_>> for Transaction {
    type Error = ();

    fn try_from(value: TransactionDTO) -> Result<Self, Self::Error> {
        let kind = match value.kind {
            "d" => TransactionKind::Debit,
            "c" => TransactionKind::Credit,
            _ => return Err(()),
        };

        let description = TransactionDescription::new(&value.description).map_err(|e| {
            eprintln!("desc error");
            ()
        })?;
        let amount = NonZeroI32::new(value.amount).ok_or(())?;

        Ok(Self {
            valor: amount,
            tipo: kind,
            descricao: description,
            realizada_em: OffsetDateTime::now_utc(),
        })
    }
}

impl From<Transaction> for TransactionDTO<'_> {
    fn from(value: Transaction) -> Self {
        let kind = match value.tipo {
            TransactionKind::Debit => "d",
            TransactionKind::Credit => "c",
        };

        let description = value.descricao.0.into();
        let amount = i32::from(value.valor);
        let created_on = Some(value.realizada_em.format(&Iso8601::DEFAULT).unwrap().into());

        Self {
            amount,
            kind,
            description,
            created_on,
        }
    }
}

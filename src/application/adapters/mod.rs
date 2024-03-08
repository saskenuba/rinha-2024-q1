use crate::domain::account::Account;
use crate::domain::transaction::{Transaction, TransactionDescription, TransactionKind};
use compact_str::{CompactString, ToCompactString};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::num::NonZeroI32;
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize)]
pub struct StatementDTO {
    #[serde(rename = "saldo")]
    saldo: SaldoDTO,
    #[serde(rename = "ultimas_transacoes")]
    transactions: Vec<TransactionDTO<'static>>,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize)]
pub struct SaldoDTO {
    #[serde(rename = "total")]
    total_balance: i32,
    #[serde(rename = "data_extrato")]
    data_extrato: CompactString,
    #[serde(rename = "limite")]
    credit_limit: u32,
}
impl StatementDTO {
    pub fn from_other(value: (Account, impl Iterator<Item = Transaction>)) -> Self {
        let formatted = OffsetDateTime::now_utc()
            .format(&Iso8601::DEFAULT)
            .unwrap()
            .to_compact_string();

        let acc = value.0;
        let transactions = value.1;
        Self {
            saldo: SaldoDTO {
                total_balance: acc.balance,
                data_extrato: formatted,
                credit_limit: acc.credit_limit,
            },
            transactions: transactions
                .into_iter()
                .map(TransactionDTO::from)
                .collect::<Vec<_>>(),
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

        let description = TransactionDescription::new(&value.description).map_err(|_| {
            eprintln!("desc error");
        })?;

        let amount = if matches!(kind, TransactionKind::Debit) {
            NonZeroI32::new(-value.amount).ok_or(())?
        } else {
            NonZeroI32::new(value.amount).ok_or(())?
        };

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
            _ => unreachable!(),
        };

        let description = value.descricao.0.into();
        let amount = i32::from(value.valor.abs());
        let created_on = Some(value.realizada_em.format(&Iso8601::DEFAULT).unwrap().into());

        Self {
            amount,
            kind,
            description,
            created_on,
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize)]
pub struct TransactionResponseDTO {
    #[serde(rename = "limite")]
    credit_limit: u32,
    #[serde(rename = "saldo")]
    balance: i32,
}

impl From<Account> for TransactionResponseDTO {
    fn from(value: Account) -> Self {
        Self {
            credit_limit: value.credit_limit,
            balance: value.balance,
        }
    }
}

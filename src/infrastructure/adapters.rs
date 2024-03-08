use crate::domain::transaction::{Transaction, TransactionDescription, TransactionKind};
use compact_str::CompactString;
use std::ffi::CStr;
use std::num::NonZeroI32;
use time::OffsetDateTime;
use zerocopy_derive::{IntoBytes, KnownLayout, NoCell, TryFromBytes};

#[derive(KnownLayout, NoCell, TryFromBytes, IntoBytes, Debug, Copy, Clone, PartialEq)]
#[repr(C)]
pub struct TransactionDAO {
    valor: i32,
    tipo: TransactionKind,
    _pad: [u8; 3],
    desc: [u8; 32],
    realizada_em: i128,
    _pad2: [u8; 8],
}

impl From<TransactionDAO> for Transaction {
    fn from(value: TransactionDAO) -> Self {
        let buf = CStr::from_bytes_until_nul(&value.desc).unwrap();
        let descricao =
            TransactionDescription(unsafe { CompactString::from_utf8_unchecked(buf.to_bytes()) });
        let valor = unsafe { NonZeroI32::new_unchecked(value.valor) };

        Self {
            valor: valor,
            tipo: value.tipo,
            descricao,
            realizada_em: OffsetDateTime::from_unix_timestamp_nanos(value.realizada_em).unwrap(),
        }
    }
}

impl From<&Transaction> for TransactionDAO {
    fn from(value: &Transaction) -> Self {
        let mut desc = [0u8; 32];
        let x = value.descricao.0.as_bytes();
        desc[..x.len()].copy_from_slice(x);

        Self {
            valor: i32::from(value.valor),
            tipo: value.tipo,
            _pad: [0; 3],
            desc: desc,
            realizada_em: value.realizada_em.unix_timestamp_nanos(),
            _pad2: [0; 8],
        }
    }
}

#![allow(unused_must_use)]

use crate::application::repositories::HasOffsetRange;
use crate::domain::account::Account;
use crate::domain::transaction::{Transaction, TransactionKind};
use crate::infrastructure::adapters::TransactionDAO;
use derive_more::{Deref, DerefMut};
use linux_futex::{AsFutex, Futex, Shared};
use memmap2::MmapRaw;
use shared_memory::Shmem;
use std::fs::OpenOptions;
use std::io::Write;
use std::mem::size_of;
use std::ops::Range;
use std::ptr::{slice_from_raw_parts, slice_from_raw_parts_mut};
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;
use zerocopy::{FromBytes, IntoBytes, TryFromBytes};

pub mod adapters;
pub mod server_impl;

/// A single futex will lock both the transactions range of memory of a single account, as the
/// account itself. By doing this we ensure that only a process has access to a account resources at a single time.
#[derive(Clone)]
pub struct TransactionIPCRepository {
    trans_mem: Arc<SharedMemory>,
    acc_mem: Arc<SharedMemory>,
    futex_mem: Arc<MmapRaw>,
}

#[derive(Deref, DerefMut)]
struct SharedMemory(Shmem);

impl SharedMemory {
    pub unsafe fn as_slice(&self) -> *const [u8] {
        slice_from_raw_parts(self.as_ptr(), self.len())
    }

    pub unsafe fn as_slice_by_range(&self, range: Range<usize>) -> &[u8] {
        &self.as_slice().as_ref().unwrap()[range]
    }

    /// Returns a mutable slice from the first byte of the shared memory.
    pub unsafe fn as_slice_mut(&self) -> &mut [u8] {
        slice_from_raw_parts_mut(self.as_ptr(), self.len())
            .as_mut()
            .unwrap()
    }

    pub unsafe fn as_slice_mut_by_range(&self, range: Range<usize>) -> &mut [u8] {
        &mut self.as_slice_mut()[range]
    }
}

unsafe impl Sync for SharedMemory {}
unsafe impl Send for SharedMemory {}

const TRANSACTION_DAO_SIZE: usize = size_of::<TransactionDAO>();

const NO_LOCK: u32 = 0;
const HAS_LOCK: u32 = 1;

impl TransactionIPCRepository {
    //noinspection RsUnresolvedReference
    /// memory repr is:
    /// [ AtomicU32 ISLOCKED USER1 | AtomicU32 READERS USER1 | USER2... ]
    pub async unsafe fn lock_account_resources(&self, account_id: i32) -> &Futex<Shared> {
        let islock_size = size_of::<AtomicU32>() * (account_id - 1) as usize;
        let readers_size = islock_size + size_of::<AtomicU32>();

        let is_locked = self
            .futex_mem
            .as_ptr()
            .cast::<AtomicU32>()
            .byte_offset(islock_size.try_into().unwrap());

        let is_accessing = self
            .futex_mem
            .as_ptr()
            .cast::<AtomicU32>()
            .byte_offset(readers_size.try_into().unwrap());

        let is_lock_atomic = is_locked.as_ref().unwrap();
        let readers_atomic = is_accessing.as_ref().unwrap();

        let futex: &Futex<Shared> = is_lock_atomic.as_futex();

        loop {
            match is_lock_atomic.compare_exchange_weak(NO_LOCK, HAS_LOCK, SeqCst, SeqCst) {
                Ok(_) => break,
                Err(_) => {
                    // don't block the executor and send a signal
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    tokio::spawn(async move {
                        futex.wait(NO_LOCK);
                        tx.send(())
                    });
                    rx.await.unwrap();
                    println!("wakened..");
                }
            }
        }

        futex
    }
    pub unsafe fn unlock(&self, futex: &Futex<Shared>, account_id: i32) {
        let islock_size = size_of::<AtomicU32>() * (account_id - 1) as usize;
        let readers_size = islock_size + size_of::<AtomicU32>();

        let is_locked = self
            .futex_mem
            .as_ptr()
            .cast::<AtomicU32>()
            .byte_offset(islock_size.try_into().unwrap());

        let is_accessing = self
            .futex_mem
            .as_ptr()
            .cast::<AtomicU32>()
            .byte_offset(readers_size.try_into().unwrap());

        let is_lock_atomic = is_locked.as_ref().unwrap();
        let readers_atomic = is_accessing.as_ref().unwrap();

        is_lock_atomic.store(NO_LOCK, SeqCst);
        readers_atomic.fetch_sub(1, SeqCst);

        futex.wake(1);
        println!("tarefa intensa finalizou..");
    }

    fn setup_files() -> MmapRaw {
        let transaction_file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open("./db/transactions")
            .unwrap();

        let account_file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open("./db/acc")
            .unwrap();

        let mut futexes_file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open("./db/super-secret.lock")
            .unwrap();

        futexes_file.set_len(1024).unwrap();
        futexes_file.write_all(&[0; 1024]);
        let futex_mmap = memmap2::MmapOptions::new().map_raw(&futexes_file).unwrap();

        futex_mmap
    }

    /// Creates the shared memory for accounts, transactions and the futex file.
    pub fn init_pool() -> Self {
        let tra_flink = "rinha-transactions";
        let acc_flink = "rinha-accounts";

        let shm_trans_size = 1024 * 4;
        let shm_trans = if std::env::var("master").is_ok() || cfg!(debug_assertions) {
            loop {
                let trans_mem = shared_memory::ShmemConf::new()
                    .size(shm_trans_size)
                    .flink(tra_flink)
                    .open();
                if let Ok(trans) = trans_mem {
                    break trans;
                }
            }
        } else {
            shared_memory::ShmemConf::new()
                .size(shm_trans_size)
                .flink(tra_flink)
                .force_create_flink()
                .create()
                .unwrap()
        };

        let shm_accs_size = 1024;
        let shm_accs = if std::env::var("master").is_ok() || cfg!(debug_assertions) {
            loop {
                let acc_mem = shared_memory::ShmemConf::new()
                    .size(shm_accs_size)
                    .flink(acc_flink)
                    .open();
                if let Ok(acc) = acc_mem {
                    break acc;
                }
            }
        } else {
            shared_memory::ShmemConf::new()
                .size(shm_accs_size)
                .flink(acc_flink)
                .force_create_flink()
                .create()
                .unwrap()
        };

        let trans_shm = SharedMemory(shm_trans);
        let acc_shm = SharedMemory(shm_accs);

        TransactionIPCRepository {
            trans_mem: trans_shm.into(),
            acc_mem: acc_shm.into(),
            futex_mem: Arc::new(Self::setup_files()),
        }
    }

    pub unsafe fn setup_db(&self, accs: &[Account]) {
        let acc_file = &self.acc_mem;

        for acc in accs {
            let acc_offset_rng = Account::offset_range(acc.id);
            let dest = acc_file.as_slice_mut()[acc_offset_rng].as_mut();
            dest.copy_from_slice(acc.as_bytes());
        }
    }

    pub unsafe fn get_acc_and_transactions(&self, account_id: i32) -> (Account, Vec<Transaction>) {
        // account
        let acc = {
            let acc_offset_range = Account::offset_range(account_id);
            let account_bytes = self.acc_mem.as_slice_by_range(acc_offset_range);
            Account::read_from(account_bytes).unwrap()
        };

        // transaction
        let tr_file = &self.trans_mem;
        let mut buffer = Vec::with_capacity(10);

        let trans_offset = Transaction::offset_range(account_id);
        let mut ptr_offseted = tr_file.as_ptr().byte_offset(trans_offset.start as isize);
        'read: loop {
            let slice = slice_from_raw_parts(ptr_offseted as *const _, TRANSACTION_DAO_SIZE);
            let res = TransactionDAO::try_read_from(slice.as_ref().unwrap()).map(Transaction::from);

            // we insert them in order, if we find an invalid, it means there is nothing past it
            // and we can skip
            if let Some(res) = res {
                if res.tipo == TransactionKind::Invalid {
                    break 'read;
                }
                buffer.push(res)
            } else {
                break;
            }
            ptr_offseted = ptr_offseted.byte_add(TRANSACTION_DAO_SIZE);
        }

        buffer.reverse();
        (acc, buffer)
    }
    pub unsafe fn add_transaction(&self, acc: &Account, transaction: &Transaction) {
        // overwrite account data
        let acc_file = &self.acc_mem;
        let acc_offset = Account::offset_range(acc.id);
        acc_file
            .as_slice_mut_by_range(acc_offset)
            .copy_from_slice(acc.as_bytes());

        // overwrite account data
        let trans_offset = Transaction::offset_range(acc.id);
        let mut buffer = Vec::with_capacity(10);
        let tr_file = &self.trans_mem;
        let mut ptr_offseted = tr_file.as_ptr().byte_offset(trans_offset.start as isize);
        'read: loop {
            let slice = slice_from_raw_parts(ptr_offseted as *const _, TRANSACTION_DAO_SIZE);
            let res = TransactionDAO::try_read_from(slice.as_ref().unwrap()).map(Transaction::from);

            if let Some(res) = res {
                if res.tipo == TransactionKind::Invalid {
                    break 'read;
                }
                buffer.push(res)
            } else {
                break;
            }
            ptr_offseted = ptr_offseted.byte_add(TRANSACTION_DAO_SIZE);
        }
        drop(ptr_offseted);

        let dao: TransactionDAO = transaction.into();
        let mut ptr_offseted_mut = tr_file.as_ptr().byte_offset(trans_offset.start as isize);

        // if none we must find the oldest one
        if buffer.len() < 10 {
            let buffer_len = buffer.len();
            ptr_offseted_mut = ptr_offseted_mut.byte_add(TRANSACTION_DAO_SIZE * buffer_len);
            ptr_offseted.copy_from_nonoverlapping(dao.as_bytes().as_ptr(), TRANSACTION_DAO_SIZE);
            return ();
        }
    }
}

/// Add a transaction to the shared memory.

#[cfg(test)]
mod tests {
    use super::*;

    fn get_pool() -> TransactionIPCRepository {
        let res = TransactionIPCRepository::init_pool();
        let accounts = [
            Account::new(1, 100_000),
            Account::new(2, 80_000),
            Account::new(3, 1_000_000),
            Account::new(4, 10_000_000),
            Account::new(5, 500_000),
        ];
        unsafe {
            res.setup_db(&accounts);
        };
        res
    }

    #[test]
    fn success_get_pool() {
        get_pool();
    }

    #[test]
    fn success_get_account() {
        let pool = get_pool();

        unsafe {
            let (acc, _trans) = pool.get_acc_and_transactions(3);
            assert_eq!(acc.id, 3);
            assert_eq!(acc.balance, 0);
            assert_eq!(acc.credit_limit, 1_000_000);
        }
    }

    #[test]
    fn success_save_transaction() {
        let pool = get_pool();

        let account = Account::new(1, 100_000);
        let tr_1 = Transaction::generate(100, "xxx");
        let tr_2 = Transaction::generate(250, "xxx");
        unsafe {
            pool.add_transaction(&account, &tr_1);
            pool.add_transaction(&account, &tr_2);
        }
    }
    #[test]
    fn success_get_transactions() {
        let pool = get_pool();

        let account = Account::new(1, 100_000);
        let tr_1 = Transaction::generate(100, "xxx");
        let acc_after_transaction = account.add_transaction(&tr_1).unwrap();
        unsafe {
            pool.add_transaction(&acc_after_transaction, &tr_1);
        }
        let (account, _) = unsafe { pool.get_acc_and_transactions(1) };
        assert_eq!(account.balance, 100);
        assert_eq!(account.credit_limit, 100_000);

        let tr_2 = Transaction::generate(-50_100, "vem de pix");
        let acc_after_transaction = account.add_transaction(&tr_2).unwrap();
        unsafe {
            pool.add_transaction(&acc_after_transaction, &tr_2);
        }

        let (account, trans) = unsafe { pool.get_acc_and_transactions(1) };
        assert_eq!(account.balance, -50_000);
        assert_eq!(account.credit_limit, 100_000);
        assert_eq!(trans.first(), Some(tr_2).as_ref());
        assert_eq!(trans.get(1), Some(tr_1).as_ref());
    }
}

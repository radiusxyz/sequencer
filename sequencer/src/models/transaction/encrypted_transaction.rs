use serde::{Deserialize, Serialize};

use crate::models::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EncryptedTransactionModel {
    encrypted_transaction: EncryptedTransaction,
    time_lock_puzzle: TimeLockPuzzle,
}

impl EncryptedTransactionModel {
    pub fn new(
        encrypted_transaction: EncryptedTransaction,
        time_lock_puzzle: TimeLockPuzzle,
    ) -> Self {
        Self {
            encrypted_transaction,
            time_lock_puzzle,
        }
    }
}

impl EncryptedTransactionModel {
    pub const ID: &'static str = stringify!(EncryptedTransactionModel);

    pub fn get(
        rollup_id: &ClusterId,
        block_height: &BlockHeight,
        transaction_order: &TransactionOrder,
    ) -> Result<Self, DbError> {
        let key = (Self::ID, rollup_id, block_height, transaction_order);
        database()?.get(&key)
    }

    pub fn put(
        &self,
        rollup_id: &ClusterId,
        block_height: &BlockHeight,
        transaction_order: &TransactionOrder,
    ) -> Result<(), DbError> {
        let key = (Self::ID, rollup_id, block_height, transaction_order);
        database()?.put(&key, self)
    }
}
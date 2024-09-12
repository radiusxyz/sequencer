use crate::types::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EncryptedTransactionModel;

impl EncryptedTransactionModel {
    pub const ID: &'static str = stringify!(EncryptedTransactionModel);

    pub fn put_with_transaction_hash(
        rollup_id: &String,
        transaction_hash: &String,

        encrypted_transaction: &EncryptedTransaction,
    ) -> Result<(), KvStoreError> {
        let key = &(Self::ID, rollup_id, transaction_hash);

        kvstore()?.put(key, encrypted_transaction)
    }

    pub fn put_with_order_commitment(
        rollup_id: &String,
        rollup_block_height: u64,
        transaction_order: u64,

        encrypted_transaction: &EncryptedTransaction,
    ) -> Result<(), KvStoreError> {
        let key = &(Self::ID, rollup_id, rollup_block_height, transaction_order);

        kvstore()?.put(key, encrypted_transaction)
    }

    pub fn get_with_transaction_hash(
        rollup_id: &String,
        transaction_hash: &String,
    ) -> Result<EncryptedTransaction, KvStoreError> {
        let key = &(Self::ID, rollup_id, transaction_hash);

        kvstore()?.get(key)
    }

    pub fn get_with_order_commitment(
        rollup_id: &String,
        block_height: u64,
        transaction_order: u64,
    ) -> Result<EncryptedTransaction, KvStoreError> {
        let key = &(Self::ID, rollup_id, block_height, transaction_order);

        kvstore()?.get(key)
    }
}

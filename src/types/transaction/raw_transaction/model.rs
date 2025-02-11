use crate::types::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RawTransactionModel;

impl RawTransactionModel {
    pub const ID: &'static str = stringify!(RawTransactionModel);

    pub fn put_with_transaction_hash(
        rollup_id: &str,
        transaction_hash: &RawTransactionHash,

        raw_transaction: RawTransaction,
        is_direct_sent: bool,
    ) -> Result<(), KvStoreError> {
        let key = &(Self::ID, rollup_id, transaction_hash);

        kvstore()?.put(key, &(raw_transaction, is_direct_sent))
    }

    pub fn put(
        rollup_id: &str,
        block_height: u64,
        transaction_order: u64,

        raw_transaction: RawTransaction,
        is_direct_sent: bool,
    ) -> Result<(), KvStoreError> {
        let key = &(Self::ID, rollup_id, block_height, transaction_order);

        kvstore()?.put(key, &(raw_transaction, is_direct_sent))
    }

    pub fn get_with_transaction_hash(
        rollup_id: &str,
        transaction_hash: &str,
    ) -> Result<(RawTransaction, bool), KvStoreError> {
        let key = &(Self::ID, rollup_id, transaction_hash);

        kvstore()?.get(key)
    }

    pub fn get(
        rollup_id: &str,
        block_height: u64,
        transaction_order: u64,
    ) -> Result<(RawTransaction, bool), KvStoreError> {
        let key = &(Self::ID, rollup_id, block_height, transaction_order);

        kvstore()?.get(key)
    }
}

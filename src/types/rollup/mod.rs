mod rollup_metadata;
mod rollup_type;
mod rollup_validation_info;

use std::collections::{btree_set, BTreeSet};

pub use rollup_metadata::*;
pub use rollup_type::*;
pub use rollup_validation_info::*;

use super::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize, Model)]
#[kvstore(key(rollup_id: &str))]
pub struct Rollup {
    pub cluster_id: String,
    pub platform: Platform,
    pub service_provider: ServiceProvider,

    pub rollup_id: String,
    pub rollup_type: RollupType,
    pub encrypted_transaction_type: EncryptedTransactionType,
    pub order_commitment_type: OrderCommitmentType,

    #[serde(serialize_with = "serialize_address")]
    pub owner: Address,

    pub validation_info: RollupValidationInfo,

    #[serde(serialize_with = "serialize_address_list")]
    pub executor_address_list: Vec<Address>,

    pub max_gas_limit: u64,
}

impl Rollup {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rollup_id: String,
        rollup_type: RollupType,
        encrypted_transaction_type: EncryptedTransactionType,

        owner: Address,
        rollup_validation_info: RollupValidationInfo,
        order_commitment_type: OrderCommitmentType,
        executor_address_list: Vec<Address>,

        cluster_id: String,

        platform: Platform,
        service_provider: ServiceProvider,
    ) -> Self {
        Self {
            rollup_id,
            rollup_type,
            encrypted_transaction_type,
            owner,
            validation_info: rollup_validation_info,
            order_commitment_type,
            executor_address_list,
            cluster_id,
            platform,
            service_provider,
            max_gas_limit: 0,
        }
    }

    pub fn set_executor_address_list(&mut self, executor_address_list: Vec<Address>) {
        self.executor_address_list = executor_address_list;
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, Model)]
#[kvstore(key())]
pub struct RollupIdList(BTreeSet<String>);

impl RollupIdList {
    pub fn insert(&mut self, cluster_id: impl AsRef<str>) {
        self.0.insert(cluster_id.as_ref().into());
    }

    pub fn remove(&mut self, cluster_id: impl AsRef<str>) {
        self.0.remove(cluster_id.as_ref());
    }

    pub fn iter(&self) -> btree_set::Iter<'_, String> {
        self.0.iter()
    }
}

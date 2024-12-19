use std::str::FromStr;

use sha3::{Digest, Sha3_256};

use crate::{error::Error, types::prelude::*};

#[derive(Clone, Debug, Deserialize, Serialize, Model)]
#[kvstore(key(rollup_id: &String, rollup_block_height: u64, transaction_order: u64))]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum OrderCommitment {
    Single(SingleOrderCommitment),
    Bundle(BundleOrderCommitment),
}

// #############################################################################

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BundleOrderCommitment {
    order_commitment_list: Vec<SingleOrderCommitment>,
    signature: Signature,
}

// #############################################################################

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum SingleOrderCommitment {
    TransactionHash(TransactionHashOrderCommitment),
    Sign(SignOrderCommitment),
}

impl Default for SingleOrderCommitment {
    fn default() -> Self {
        Self::TransactionHash(TransactionHashOrderCommitment::default())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OrderCommitmentType {
    TransactionHash,
    Sign,
}

impl FromStr for OrderCommitmentType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "transaction_hash" | "TransactionHash" => Ok(Self::TransactionHash),
            "sign" | "Sign" => Ok(Self::Sign),
            _ => Err(Error::UnsupportedOrderCommitmentType),
        }
    }
}

// #############################################################################

#[derive(Clone, Default, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct TransactionHashOrderCommitment(pub String);

// #############################################################################

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SignOrderCommitment {
    pub data: OrderCommitmentData,
    pub signature: String,
}

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
pub struct OrderCommitmentData {
    pub rollup_id: String,
    pub block_height: u64,
    pub transaction_order: u64,

    #[serde(serialize_with = "serialize_merkle_node_list")]
    pub pre_merkle_path: Vec<[u8; 32]>,
}

// #############################################################################

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OrderHashList(Vec<OrderHash>);

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct OrderHash(String);

impl Default for OrderHash {
    fn default() -> Self {
        Self(const_hex::encode_prefixed([0; 32]))
    }
}

impl From<String> for OrderHash {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl OrderHash {
    pub fn update_order_hash(&self, raw_tx_hash: &RawTransactionHash) -> OrderHash {
        let mut hasher = Sha3_256::new();
        hasher.update(self.0.as_bytes());
        hasher.update(raw_tx_hash);
        let order_hash_bytes = hasher.finalize();

        OrderHash(const_hex::encode_prefixed(order_hash_bytes))
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

use std::str::FromStr;

use ethers::utils::hex;
use sha3::{Digest, Sha3_256};

use crate::{error::Error, types::prelude::*};

mod model;
pub use model::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
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
#[serde(tag = "type", content = "data")]
#[serde(rename_all = "snake_case")]
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
            _ => Err(Error::NotSupportedOrderCommitmentType),
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
    pub signature: Signature,
}

impl Default for SignOrderCommitment {
    fn default() -> Self {
        Self {
            data: OrderCommitmentData::default(),
            signature: Signature::from(Vec::new()),
        }
    }
}

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
pub struct OrderCommitmentData {
    pub rollup_id: String,
    pub block_height: u64,
    pub transaction_order: u64,
    pub previous_order_hash: OrderHash,
}

// #############################################################################

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OrderHashList(Vec<OrderHash>);

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct OrderHash(String);

impl OrderHash {
    pub fn update_order_hash(&self, raw_tx_hash: &RawTransactionHash) -> OrderHash {
        let mut hasher = Sha3_256::new();

        // TODO(jaemin): check hasher params
        hasher.update(self.0.as_bytes());
        hasher.update(raw_tx_hash.clone().into_inner().as_bytes());

        let order_hash_bytes = hasher.finalize();
        OrderHash(hex::encode(order_hash_bytes))
    }
}

impl Default for OrderHash {
    fn default() -> Self {
        Self("0000000000000000000000000000000000000000000000000000000000000000".to_owned())
    }
}

mod block;
mod cluster;
mod config;
mod merkle;
mod order_commitment;
mod rollup;
mod sequencing;
mod time_lock_puzzle;
mod transaction;
mod validation;
mod zkp;

pub use block::*;
pub use cluster::*;
pub use config::*;
pub use merkle::*;
pub use order_commitment::*;
use radius_sdk::signature::Address;
pub use rollup::*;
pub use sequencing::*;
use serde::ser::SerializeSeq;
pub use time_lock_puzzle::*;
pub use transaction::*;
pub use validation::*;
pub use zkp::*;

pub(crate) mod prelude {
    pub use radius_sdk::{
        kvstore::{kvstore, KvStoreError, Lock, Model},
        signature::{Address, Signature},
    };
    pub use serde::{Deserialize, Serialize};

    pub use crate::types::*;
}

pub fn serialize_address<S>(address: &Address, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&address.as_hex_string())
}

fn serialize_address_list<S>(addresses: &Vec<Address>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut seq = serializer.serialize_seq(Some(addresses.len()))?;
    for address in addresses {
        seq.serialize_element(&address.as_hex_string())?;
    }
    seq.end()
}

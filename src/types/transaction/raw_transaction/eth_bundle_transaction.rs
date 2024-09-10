use ethers::utils::hex;

use crate::types::prelude::*;

// TODO: stompesi
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EthRawBundleTransaction(String);

impl From<String> for EthRawBundleTransaction {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl EthRawBundleTransaction {
    pub fn raw_transaction_hash(&self) -> RawTransactionHash {
        let raw_transaction_string = serde_json::to_string(&self.0).unwrap();
        let parsed_raw_transaction_string: String =
            serde_json::from_str(&raw_transaction_string).unwrap();
        let decoded_transaction = decode_rlp_transaction(&parsed_raw_transaction_string).unwrap();

        RawTransactionHash::new(format!(
            "0x{}",
            hex::encode(decoded_transaction.hash.as_bytes())
        ))
    }
}
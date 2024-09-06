use super::prelude::*;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RollupMetadata {
    block_height: u64,
    transaction_order: u64,
    order_hash: OrderHash,
}

impl RollupMetadata {
    pub fn block_height(&self) -> u64 {
        self.block_height
    }

    pub fn set_block_height(&mut self, block_height: u64) {
        self.block_height = block_height;
    }

    pub fn transaction_order(&self) -> u64 {
        self.transaction_order
    }

    pub fn order_hash(&self) -> OrderHash {
        self.order_hash.clone()
    }

    pub fn issue_transaction_order(&mut self) -> u64 {
        self.transaction_order += 1;

        self.transaction_order
    }

    /// Return the current [`OrderHash`].
    pub fn issue_order_hash(&mut self, raw_transaction_hash: &RawTransactionHash) -> OrderHash {
        let current_order_hash = self.order_hash.clone();
        self.order_hash = self.order_hash.issue_order_hash(raw_transaction_hash);

        current_order_hash
    }

    /// Return the current [`RollupMetadata`].
    pub fn issue_rollup_metadata(&mut self, new_block_height: u64) -> Self {
        let current_block_metadata = self.clone();

        self.block_height = new_block_height;
        self.transaction_order = 0;
        self.order_hash = OrderHash::default();

        current_block_metadata
    }
}

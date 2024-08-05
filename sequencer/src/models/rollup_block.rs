use super::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RollupBlockModel {
    pub rollup_block: Block,
}

impl RollupBlockModel {
    const ID: &'static str = stringify!(RollupBlockModel);

    pub fn get(rollup_id: &ClusterId, rollup_block_height: &BlockHeight) -> Result<Self, DbError> {
        let key = (Self::ID, rollup_id, rollup_block_height);
        database()?.get(&key)
    }

    pub fn put(
        &self,
        rollup_id: &ClusterId,
        rollup_block_height: &BlockHeight,
    ) -> Result<(), DbError> {
        let key = (Self::ID, rollup_id, rollup_block_height);
        database()?.put(&key, self)
    }
}

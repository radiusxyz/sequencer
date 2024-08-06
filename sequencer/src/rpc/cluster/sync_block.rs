use crate::{models::ClusterMetadataModel, rpc::prelude::*};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncBlock {
    pub rollup_id: ClusterId,
    pub liveness_block_height: BlockHeight,
    pub rollup_block_height: BlockHeight,
    pub transaction_order: TransactionOrder,
}

impl SyncBlock {
    pub const METHOD_NAME: &'static str = stringify!(SyncBuildBlock);

    pub async fn handler(parameter: RpcParameter, context: Arc<AppState>) -> Result<(), RpcError> {
        let parameter = parameter.parse::<Self>()?;

        let cluster = context.get_cluster(&parameter.rollup_id).await?;

        // let cluster = context.cluster().await?;

        match ClusterMetadataModel::get_mut(&parameter.rollup_id) {
            Ok(mut cluster_metadata_model) => {
                // builder::build_block(
                //     context.ssal_client(),
                //     cluster,
                //     parameter.rollup_id,
                //     parameter.rollup_block_height,
                //     parameter.transaction_order,
                //     false,
                // );
            }
            Err(error) => {}
        }

        Ok(())
    }
}

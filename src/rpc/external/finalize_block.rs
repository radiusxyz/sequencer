use crate::rpc::{cluster::SyncBlock, prelude::*};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FinalizeBlockMessage {
    // platform: Platform,
    // service_provider: ServiceProvider,
    // cluster_id: String,
    // chain_type: ChainType,
    // address: Vec<u8>,
    rollup_id: String,
    liveness_block_height: u64,
    rollup_block_height: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FinalizeBlock {
    pub message: FinalizeBlockMessage,
    pub signature: Signature,
}

impl FinalizeBlock {
    pub const METHOD_NAME: &'static str = "finalize_block";

    pub async fn handler(parameter: RpcParameter, _context: Arc<AppState>) -> Result<(), RpcError> {
        let parameter = parameter.parse::<Self>()?;

        // // get rollup info for address and chain type
        // // verify siganture
        // parameter.signature.verify_signature(
        //     serialize_to_bincode(&parameter.message)?.as_slice(),
        //     parameter.message.address.as_slice(),
        //     parameter.message.chain_type,
        // )?;

        match RollupMetadataModel::get_mut(&parameter.message.rollup_id) {
            Ok(mut rollup_metadata) => {
                let current_rollup_metadata =
                    rollup_metadata.issue_rollup_metadata(parameter.message.rollup_block_height);
                rollup_metadata.update()?;

                let cluster_info = ClusterInfoModel::get(
                    parameter.message.liveness_block_height,
                    &parameter.message.rollup_id,
                )?;

                if cluster_info.sequencer_list().is_empty() {
                    return Err(Error::EmptySequencerList.into());
                }

                let cluster_metadata = ClusterMetadata::new(
                    cluster_info.sequencer_list().len()
                        % parameter.message.rollup_block_height as usize,
                    cluster_info.my_index(),
                    cluster_info.sequencer_list().clone(),
                );
                ClusterMetadataModel::put(
                    &parameter.message.rollup_id,
                    parameter.message.rollup_block_height,
                    &cluster_metadata,
                )?;

                // Sync.
                Self::sync_block(
                    &parameter,
                    current_rollup_metadata.transaction_order(),
                    cluster_metadata,
                );
            }
            Err(error) => {
                if error.is_none_type() {
                    let mut rollup_metadata = RollupMetadata::default();
                    rollup_metadata.set_block_height(parameter.message.rollup_block_height);
                    RollupMetadataModel::put(&parameter.message.rollup_id, &rollup_metadata)?;

                    let cluster_info = ClusterInfoModel::get(
                        parameter.message.liveness_block_height,
                        &parameter.message.rollup_id,
                    )?;

                    if cluster_info.sequencer_list().is_empty() {
                        return Err(Error::EmptySequencerList.into());
                    }

                    let cluster_metadata = ClusterMetadata::new(
                        cluster_info.sequencer_list().len()
                            % parameter.message.rollup_block_height as usize,
                        cluster_info.my_index(),
                        cluster_info.sequencer_list().clone(),
                    );
                    ClusterMetadataModel::put(
                        &parameter.message.rollup_id,
                        parameter.message.rollup_block_height,
                        &cluster_metadata,
                    )?;

                    // Sync.
                    Self::sync_block(
                        &parameter,
                        rollup_metadata.transaction_order(),
                        cluster_metadata,
                    );
                } else {
                    return Err(error.into());
                }
            }
        }

        Ok(())
    }

    pub fn sync_block(parameter: &Self, transaction_order: u64, cluster_metadata: ClusterMetadata) {
        let parameter = parameter.clone();

        tokio::spawn(async move {
            let rpc_parameter = SyncBlock {
                rollup_id: parameter.message.rollup_id.clone(),
                liveness_block_height: parameter.message.liveness_block_height,
                rollup_block_height: parameter.message.rollup_block_height,
                transaction_order,
            };

            for sequencer_rpc_url in cluster_metadata.others() {
                let rpc_parameter = rpc_parameter.clone();

                if let Some(sequencer_rpc_url) = sequencer_rpc_url {
                    tokio::spawn(async move {
                        let client = RpcClient::new(sequencer_rpc_url).unwrap();
                        let _ = client
                            .request::<SyncBlock, ()>(SyncBlock::METHOD_NAME, rpc_parameter.clone())
                            .await;
                    });
                }
            }
        });
    }
}
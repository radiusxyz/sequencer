use crate::{
    rpc::{
        cluster::{SyncEncryptedTransaction, SyncEncryptedTransactionMessage},
        prelude::*,
    },
    types::*,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SendEncryptedTransaction {
    pub rollup_id: String,
    pub encrypted_transaction: EncryptedTransaction,
}

impl SendEncryptedTransaction {
    pub const METHOD_NAME: &'static str = "send_encrypted_transaction";

    pub async fn handler(
        parameter: RpcParameter,
        context: Arc<AppState>,
    ) -> Result<OrderCommitment, RpcError> {
        let parameter = parameter.parse::<Self>()?;
        let rollup = Rollup::get(&parameter.rollup_id)?;

        tracing::info!(
            "Send encrypted transaction - rollup id: {:?}, encrypted transaction: {:?}",
            parameter.rollup_id,
            parameter.encrypted_transaction
        );

        // 1. Check supported encrypted transaction
        check_supported_encrypted_transaction(&rollup, &parameter.encrypted_transaction)?;

        // 2. Check is leader
        let mut rollup_metadata = RollupMetadata::get_mut(&parameter.rollup_id)?;

        let platform = rollup.platform;
        let service_provider = rollup.service_provider;
        let cluster_id = &rollup_metadata.cluster_id;
        let rollup_block_height = rollup_metadata.rollup_block_height;

        let cluster = Cluster::get(
            platform,
            service_provider,
            cluster_id,
            rollup_metadata.platform_block_height,
        )?;

        if rollup_metadata.is_leader {
            let (transaction_order, pre_merkle_path) = rollup_metadata.add_transaction_hash(
                parameter
                    .encrypted_transaction
                    .raw_transaction_hash()
                    .as_ref(),
            );

            rollup_metadata.update()?;

            let order_commitment = issue_order_commitment(
                context.clone(),
                rollup.platform,
                parameter.rollup_id.clone(),
                rollup.order_commitment_type,
                parameter.encrypted_transaction.raw_transaction_hash(),
                rollup_block_height,
                transaction_order,
                pre_merkle_path.clone(),
            )
            .await?;

            let transaction_hash = parameter.encrypted_transaction.raw_transaction_hash();

            EncryptedTransactionModel::put_with_transaction_hash(
                &parameter.rollup_id,
                &transaction_hash,
                &parameter.encrypted_transaction,
            )?;

            EncryptedTransactionModel::put(
                &parameter.rollup_id,
                rollup_block_height,
                transaction_order,
                &parameter.encrypted_transaction,
            )?;

            order_commitment.put(&parameter.rollup_id, rollup_block_height, transaction_order)?;

            // Sync Transaction
            sync_encrypted_transaction(
                cluster,
                context.clone(),
                rollup.platform,
                parameter.rollup_id.clone(),
                rollup_block_height,
                transaction_order,
                parameter.encrypted_transaction.clone(),
                order_commitment.clone(),
            );

            Ok(order_commitment)
        } else {
            drop(rollup_metadata);
            let leader_external_rpc_url =
                cluster.get_leader_external_rpc_url(rollup_block_height)?;

            let rpc_client = RpcClient::new()?;
            match rpc_client
                .request(
                    leader_external_rpc_url,
                    SendEncryptedTransaction::METHOD_NAME,
                    &parameter,
                    Id::Null,
                )
                .await
            {
                Ok(response) => Ok(response),
                Err(error) => {
                    tracing::error!(
                        "Send encrypted transaction - leader external rpc error: {:?}",
                        error
                    );
                    Err(error.into())
                }
            }
        }
    }
}

fn check_supported_encrypted_transaction(
    rollup: &Rollup,
    encrypted_transaction: &EncryptedTransaction,
) -> Result<(), Error> {
    match rollup.encrypted_transaction_type {
        EncryptedTransactionType::Pvde => {
            if !matches!(encrypted_transaction, EncryptedTransaction::Pvde(_)) {
                return Err(Error::UnsupportedEncryptedMempool);
            }
        }
        EncryptedTransactionType::Skde => {
            if !matches!(encrypted_transaction, EncryptedTransaction::Skde(_)) {
                return Err(Error::UnsupportedEncryptedMempool);
            }
        }
        EncryptedTransactionType::NotSupport => return Err(Error::UnsupportedEncryptedMempool),
    };

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn sync_encrypted_transaction(
    cluster: Cluster,
    context: Arc<AppState>,
    platform: Platform,
    rollup_id: String,
    rollup_block_height: u64,
    transaction_order: u64,
    encrypted_transaction: EncryptedTransaction,
    order_commitment: OrderCommitment,
) {
    tokio::spawn(async move {
        let follower_cluster_rpc_url_list: Vec<String> =
            cluster.get_follower_cluster_rpc_url_list(rollup_block_height);

        if !follower_cluster_rpc_url_list.is_empty() {
            let message = SyncEncryptedTransactionMessage {
                rollup_id,
                rollup_block_height,
                transaction_order,
                encrypted_transaction,
                order_commitment,
            };
            let signature = context
                .get_signer(platform)
                .await
                .unwrap()
                .sign_message(&message)
                .unwrap();
            let rpc_parameter = SyncEncryptedTransaction { message, signature };

            let rpc_client = RpcClient::new().unwrap();
            rpc_client
                .multicast(
                    follower_cluster_rpc_url_list,
                    SyncEncryptedTransaction::METHOD_NAME,
                    &rpc_parameter,
                    Id::Null,
                )
                .await;
        }
    });
}

#[allow(clippy::too_many_arguments)]
pub async fn issue_order_commitment(
    context: Arc<AppState>,
    platform: Platform,
    rollup_id: String,
    order_commitment_type: OrderCommitmentType,
    transaction_hash: RawTransactionHash,
    rollup_block_height: u64,
    transaction_order: u64,
    pre_merkle_path: Vec<[u8; 32]>,
) -> Result<OrderCommitment, RpcError> {
    match order_commitment_type {
        OrderCommitmentType::TransactionHash => Ok(OrderCommitment::Single(
            SingleOrderCommitment::TransactionHash(TransactionHashOrderCommitment(
                transaction_hash.as_string(),
            )),
        )),
        OrderCommitmentType::Sign => {
            let signer = context.get_signer(platform).await?;
            let order_commitment_data = OrderCommitmentData {
                rollup_id,
                block_height: rollup_block_height,
                transaction_order,
                pre_merkle_path: pre_merkle_path,
            };
            let order_commitment = SignOrderCommitment {
                data: order_commitment_data.clone(),
                signature: signer.sign_message(&order_commitment_data)?.as_hex_string(),
            };

            Ok(OrderCommitment::Single(SingleOrderCommitment::Sign(
                order_commitment,
            )))
        }
    }
}

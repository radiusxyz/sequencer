use std::{collections::HashMap, sync::Arc};

use radius_sdk::{
    json_rpc::client::{Id, RpcClient, RpcClientError},
    signature::Address,
};
use skde::delay_encryption::{decrypt, SkdeParams};
use tokio::time::{sleep, Duration};

use crate::{
    client::{liveness::distributed_key_generation::DistributedKeyGenerationClient, validation},
    error::Error,
    rpc::external::GetEncryptedTransactionWithOrderCommitment,
    state::AppState,
    types::*,
};
/// Block-builder task implements block-building mechanism for different
/// transaction types in the following order:
///
/// 1. Iterate over transactions for a given rollup ID and the block height.
/// 2. Fetch missing transactions.
/// 3. - PVDE => Decrypt missing raw transactions from other sequencers.
///    - SKDE => Decrypt the transaction with a decryption key.
/// 4. Build the block with the list of raw transactions.
/// 5. (Leader) Submit the block commitment.
pub fn block_builder(
    context: Arc<AppState>,
    rollup_id: String,
    block_creator_address: Address,
    rollup_encrypted_transaction_type: EncryptedTransactionType,
    rollup_block_height: u64,
    transaction_count: u64,
    cluster: Cluster,
) {
    tracing::info!(
        "Build block - rollup id: {:?}, block number: {:?}, transaction count: {:?}",
        rollup_id,
        rollup_block_height,
        transaction_count
    );

    match rollup_encrypted_transaction_type {
        EncryptedTransactionType::Pvde => {}
        EncryptedTransactionType::Skde => {
            block_builder_skde(
                context,
                rollup_id,
                block_creator_address,
                rollup_block_height,
                transaction_count,
                cluster,
            );
        }
        EncryptedTransactionType::NotSupport => unimplemented!(),
    }
}

pub fn block_builder_skde(
    context: Arc<AppState>,
    rollup_id: String,
    block_creator_address: Address,
    rollup_block_height: u64,
    transaction_count: u64,
    cluster: Cluster,
) {
    let distributed_key_generation_client = context.distributed_key_generation_client().clone();

    tokio::spawn(async move {
        let mut raw_transaction_list =
            Vec::<RawTransaction>::with_capacity(transaction_count as usize);
        let mut encrypted_transaction_list =
            Vec::<Option<EncryptedTransaction>>::with_capacity(transaction_count as usize);
        let mut merkle_tree = MerkleTree::new();

        let mut decryption_keys: HashMap<u64, String> = HashMap::new();

        if transaction_count > 0 {
            // 1. Iterate over transactions for a given rollup ID and the block height.
            for transaction_order in 0..transaction_count {
                let mut encrypted_transaction = match EncryptedTransactionModel::get(
                    &rollup_id,
                    rollup_block_height,
                    transaction_order,
                ) {
                    Ok(encrypted_transaction) => Some(encrypted_transaction),
                    Err(error) => {
                        if error.is_none_type() {
                            None
                        } else {
                            panic!("block_builder: {:?}", error);
                        }
                    }
                };

                encrypted_transaction_list.push(encrypted_transaction.clone());

                match RawTransactionModel::get(&rollup_id, rollup_block_height, transaction_order) {
                    // If raw transaction exists, add it to the raw transaction list.
                    Ok(raw_transaction) => {
                        raw_transaction_list.push(raw_transaction);
                    }
                    Err(error) => {
                        if error.is_none_type() {
                            if encrypted_transaction.is_none() {
                                // 2. Fetch the missing transaction from other sequencers.
                                encrypted_transaction = Some(
                                    fetch_missing_transaction(
                                        rollup_id.clone(),
                                        rollup_block_height,
                                        transaction_order,
                                        cluster.clone(),
                                    )
                                    .await
                                    .unwrap(),
                                );
                            }

                            match encrypted_transaction.unwrap() {
                                EncryptedTransaction::Skde(skde_encrypted_transaction) => {
                                    let (raw_transaction, _plain_data) = decrypt_skde_transaction(
                                        &skde_encrypted_transaction,
                                        distributed_key_generation_client.clone(),
                                        &mut decryption_keys,
                                        context.skde_params(),
                                    )
                                    .await
                                    .unwrap();

                                    merkle_tree
                                        .add_data(raw_transaction.raw_transaction_hash().as_ref());

                                    RawTransactionModel::put(
                                        &rollup_id,
                                        rollup_block_height,
                                        transaction_order,
                                        &raw_transaction,
                                    )
                                    .unwrap();

                                    raw_transaction_list.push(raw_transaction);
                                }
                                _ => {
                                    panic!("error: {:?}", error);
                                }
                            }
                        }
                    }
                }
            }
        }

        let rollup = Rollup::get(&rollup_id).unwrap();

        let signer = context.get_signer(rollup.platform()).await.unwrap();
        let sequencer_address = signer.address().clone();
        let signature = signer.sign_message("").unwrap(); // TODO: set the message.

        merkle_tree.finalize_tree();
        let block_commitment = merkle_tree.get_merkle_root();

        let block = Block::new(
            rollup_block_height,
            encrypted_transaction_list.clone(),
            raw_transaction_list.clone(),
            signature,
            BlockCommitment::from(block_commitment),
            block_creator_address.clone(),
        );

        Block::put(&block, &rollup_id, rollup_block_height).unwrap();

        if sequencer_address == block_creator_address {
            let rollup = Rollup::get(&rollup_id).unwrap();
            let rollup_validation_info = rollup.validation_info();

            let validation_info = ValidationInfoPayload::get(
                rollup_validation_info.platform(),
                rollup_validation_info.validation_service_provider(),
            )
            .unwrap();

            // TODO: Remove?
            if rollup_block_height % 100 == 0 {
                match validation_info {
                    // TODO: we have to manage the nonce for the register block commitment.
                    ValidationInfoPayload::EigenLayer(_) => {
                        let validation_client: validation::eigenlayer::ValidationClient = context
                            .get_validation_client(
                                rollup_validation_info.platform(),
                                rollup_validation_info.validation_service_provider(),
                            )
                            .await
                            .unwrap();

                        validation_client
                            .publisher()
                            .register_block_commitment(
                                rollup.cluster_id(),
                                rollup.rollup_id(),
                                rollup_block_height,
                                block_commitment,
                            )
                            .await
                            .unwrap();
                    }
                    ValidationInfoPayload::Symbiotic(_) => {
                        let validation_client: validation::symbiotic::ValidationClient = context
                            .get_validation_client(
                                rollup_validation_info.platform(),
                                rollup_validation_info.validation_service_provider(),
                            )
                            .await
                            .unwrap();

                        for _ in 0..10 {
                            match validation_client
                                .publisher()
                                .register_block_commitment(
                                    rollup.cluster_id(),
                                    rollup.rollup_id(),
                                    rollup_block_height,
                                    block_commitment,
                                )
                                .await
                                .map_err(|error| error.to_string())
                            {
                                Ok(transaction_hash) => {
                                    tracing::info!(
                                        "Registered block commitment - transaction hash: {:?}",
                                        transaction_hash
                                    );
                                    break;
                                }
                                Err(error) => {
                                    tracing::warn!("{:?}", error);
                                    sleep(Duration::from_secs(2)).await;
                                }
                            }
                        }
                    }
                }
            }
        }
    });
}

async fn decrypt_skde_transaction(
    skde_encrypted_transaction: &SkdeEncryptedTransaction,
    distributed_key_generation_client: DistributedKeyGenerationClient,
    decryption_keys: &mut HashMap<u64, String>,
    skde_params: &SkdeParams,
) -> Result<(RawTransaction, PlainData), Error> {
    let decryption_key_id = skde_encrypted_transaction.key_id();

    let decryption_key = if let std::collections::hash_map::Entry::Vacant(e) =
        decryption_keys.entry(decryption_key_id)
    {
        tracing::info!("key_id: {:?}", skde_encrypted_transaction.key_id());

        let get_decryption_key_response = distributed_key_generation_client
            .get_decryption_key(skde_encrypted_transaction.key_id())
            .await?;

        e.insert(get_decryption_key_response.decryption_key.clone());
        get_decryption_key_response.decryption_key
    } else {
        decryption_keys.get(&decryption_key_id).unwrap().clone()
    };

    match skde_encrypted_transaction.transaction_data() {
        TransactionData::Eth(transaction_data) => {
            let encrypted_data = transaction_data.encrypted_data().clone();

            let decrypted_data =
                decrypt(skde_params, encrypted_data.as_ref(), &decryption_key).unwrap();

            let eth_plain_data: EthPlainData = serde_json::from_str(&decrypted_data).unwrap();

            let rollup_transaction = transaction_data
                .open_data()
                .convert_to_rollup_transaction(&eth_plain_data);

            let eth_raw_transaction = EthRawTransaction::from(to_raw_tx(rollup_transaction));
            let raw_transaction = RawTransaction::from(eth_raw_transaction);

            Ok((raw_transaction, PlainData::from(eth_plain_data)))
        }
        TransactionData::EthBundle(_data) => {
            unimplemented!()
        }
    }
}

// TODO: Add fetch function to fetch missing transactions.
async fn fetch_missing_transaction(
    rollup_id: String,
    rollup_block_height: u64,
    transaction_order: u64,
    cluster: Cluster,
) -> Result<EncryptedTransaction, RpcClientError> {
    let others_external_rpc_url_list = cluster.get_others_external_rpc_url_list();

    let parameter = GetEncryptedTransactionWithOrderCommitment {
        rollup_id,
        rollup_block_height,
        transaction_order,
    };

    let rpc_client = RpcClient::new()?;
    let rpc_response = rpc_client
        .fetch(
            others_external_rpc_url_list,
            GetEncryptedTransactionWithOrderCommitment::METHOD_NAME,
            &parameter,
            Id::Null,
        )
        .await?;

    Ok(rpc_response)
}

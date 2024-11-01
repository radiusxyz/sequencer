use tracing::info;

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

        info!(
            "Send encrypted transaction - rollup id: {:?}, encrypted transaction: {:?}",
            parameter.rollup_id, parameter.encrypted_transaction
        );

        // 1. Check supported encrypted transaction
        check_supported_encrypted_transaction(&rollup, &parameter.encrypted_transaction)?;

        // 2. Check is leader
        let mut rollup_metadata = RollupMetadata::get_mut(&parameter.rollup_id)?;
        let platform = rollup.platform();
        let service_provider = rollup.service_provider();
        let cluster_id = rollup_metadata.cluster_id();
        let platform_block_height = rollup_metadata.platform_block_height();
        let rollup_block_height = rollup_metadata.rollup_block_height();

        let cluster = Cluster::get(
            platform,
            service_provider,
            cluster_id,
            platform_block_height,
        )?;

        if rollup_metadata.is_leader() {
            let transaction_order = rollup_metadata.transaction_order();
            rollup_metadata.increase_transaction_order();
            let previous_order_hash = rollup_metadata
                .update_order_hash(&parameter.encrypted_transaction.raw_transaction_hash());
            let current_order_hash = rollup_metadata.order_hash();
            rollup_metadata.update()?;

            let order_commitment = issue_order_commitment(
                context.clone(),
                rollup.platform(),
                parameter.rollup_id.clone(),
                rollup.order_commitment_type(),
                parameter.encrypted_transaction.raw_transaction_hash(),
                rollup_block_height,
                transaction_order,
                previous_order_hash,
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

            // Temporary block commitment
            BlockCommitment::put(
                &current_order_hash.clone().into(),
                &parameter.rollup_id,
                rollup_block_height,
                transaction_order,
            )?;

            // Sync Transaction
            sync_encrypted_transaction(
                cluster,
                context.clone(),
                rollup.platform(),
                parameter.rollup_id.clone(),
                rollup_block_height,
                transaction_order,
                parameter.encrypted_transaction.clone(),
                order_commitment.clone(),
                current_order_hash,
            );

            match parameter.encrypted_transaction {
                EncryptedTransaction::Pvde(_pvde_encrypted_transaction) => {
                    // TODO
                    // let raw_transaction = decrypt_transaction(
                    //     parameter.encrypted_transaction.clone(),
                    //     pvde_encrypted_transaction.time_lock_puzzle().
                    // clone(),     context.config().
                    // is_using_zkp(),
                    //     &Some(PvdeParams::default()),
                    // )?;

                    // RawTransactionModel::put(
                    //     &parameter.rollup_id,
                    //     rollup_block_height,
                    //     transaction_order,
                    //     raw_transaction,
                    // )?;

                    // sync_raw_transaction(
                    //     parameter.rollup_id.clone(),
                    //     parameter.encrypted_transaction.clone(),
                    //     order_commitment.clone(),
                    //     rollup_block_height,
                    //     transaction_order,
                    //     follower_rpc_url_list,
                    // );
                }
                EncryptedTransaction::Skde(_send_encrypted_transaction) => {}
            };

            Ok(order_commitment)
        } else {
            let leader_rpc_url = cluster
                .get_leader_rpc_url(rollup_block_height)
                .ok_or(Error::EmptyLeaderRpcUrl)?;
            let rpc_client = RpcClient::new()?;
            let response = rpc_client
                .request(
                    leader_rpc_url,
                    SendEncryptedTransaction::METHOD_NAME,
                    &parameter,
                    Id::Null,
                )
                .await?;

            Ok(response)
        }
    }
}

fn check_supported_encrypted_transaction(
    rollup: &Rollup,
    encrypted_transaction: &EncryptedTransaction,
) -> Result<(), Error> {
    match rollup.encrypted_transaction_type() {
        EncryptedTransactionType::Pvde => {
            if !matches!(encrypted_transaction, EncryptedTransaction::Pvde(_)) {
                return Err(Error::NotSupportEncryptedMempool);
            }
        }
        EncryptedTransactionType::Skde => {
            if !matches!(encrypted_transaction, EncryptedTransaction::Skde(_)) {
                return Err(Error::NotSupportEncryptedMempool);
            }
        }
        EncryptedTransactionType::NotSupport => return Err(Error::NotSupportEncryptedMempool),
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
    order_hash: OrderHash,
) {
    tokio::spawn(async move {
        let follower_list: Vec<String> = cluster
            .get_follower_rpc_url_list(rollup_block_height)
            .into_iter()
            .filter_map(|rpc_url| rpc_url)
            .collect();

        if !follower_list.is_empty() {
            let message = SyncEncryptedTransactionMessage {
                rollup_id,
                rollup_block_height,
                transaction_order,
                encrypted_transaction,
                order_commitment,
                order_hash,
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
                    follower_list,
                    SyncEncryptedTransaction::METHOD_NAME,
                    &rpc_parameter,
                    Id::Null,
                )
                .await;
        }
    });
}

// pub fn decrypt_transaction(
//     encrypted_transaction: EncryptedTransaction,
//     time_lock_puzzle: TimeLockPuzzle,
//     is_using_zkp: bool,
//     pvde_params: &Option<PvdeParams>,
// ) -> Result<RawTransaction, Error> {
//     let time_lock_puzzle = time_lock_puzzle.clone();
//     let encrypted_data = encrypted_transaction.encrypted_data().clone();
//     let open_data = encrypted_transaction.open_data().clone();

//     let o = BigUint::from_str(time_lock_puzzle.o()).unwrap();
//     let t = time_lock_puzzle.t();
//     let n = BigUint::from_str(time_lock_puzzle.n()).unwrap();
//     let solved_k = solve_time_lock_puzzle(o, t, n);
//     let solved_k_hash_value = hash::hash(solved_k.clone());

//     let decrypted_data = poseidon_encryption::decrypt(
//         encrypted_data.clone().into_inner().as_str(),
//         &solved_k_hash_value,
//     );

//     // TODO(jaemin): verify zkp(modify pvde library)
//     match is_using_zkp {
//         true => {
//             let pvde_params = pvde_params.as_ref().unwrap();
//             let key_validation_zkp_param = pvde_params
//                 .key_validation_zkp_param()
//                 .as_ref()
//                 .unwrap()
//                 .clone();
//             let key_validation_verify_key = pvde_params
//                 .key_validation_verifying_key()
//                 .as_ref()
//                 .unwrap()
//                 .clone();

//             let poseidon_encryption_zkp_param = pvde_params
//                 .poseidon_encryption_zkp_param()
//                 .as_ref()
//                 .unwrap()
//                 .clone();

//             let poseidon_encryption_verify_key = pvde_params
//                 .poseidon_encryption_verifying_key()
//                 .as_ref()
//                 .unwrap()
//                 .clone();

//             let time_lock_puzzle_param = pvde_params
//                 .time_lock_puzzle_param()
//                 .as_ref()
//                 .unwrap()
//                 .clone();

//             let pvde_zkp = encrypted_transaction.pvde_zkp().unwrap();

//             let sigma_protocol_public_input =
//                 pvde_zkp.public_input().to_sigma_protocol_public_input();

//             let sigma_protocol_param = SigmaProtocolParam {
//                 n: time_lock_puzzle_param.n.clone(),
//                 g: time_lock_puzzle_param.g.clone(),
//                 y_two: time_lock_puzzle_param.y_two.clone(),
//             };
//             let is_valid =
//                 verify_sigma_protocol(&sigma_protocol_public_input,
// &sigma_protocol_param);

//             if !is_valid {
//                 return Err(Error::PvdeZkpInvalid);
//             }
//             // log::info!("Done verify_sigma_protocol: {:?}", is_valid);

//             let key_validation_public_input =
//                 pvde_zkp.public_input().to_key_validation_public_input();
//             // let key_validation_public_input = KeyValidationPublicInput {
//             //     k_two: pvde_zkp.public_input.k_two.clone(),
//             //     k_hash_value: pvde_zkp.public_input.k_hash_value.clone(),
//             // };
//             let is_valid = verify_key_validation(
//                 &key_validation_zkp_param,
//                 &key_validation_verify_key,
//                 &key_validation_public_input,
//                 &pvde_zkp.time_lock_puzzle_proof().clone().into_inner(),
//             );

//             if !is_valid {
//                 return Err(Error::PvdeZkpInvalid);
//             }
//             // log::info!("Done verify_key_validation: {:?}", is_valid);

//             let poseidon_encryption_public_input =
// PoseidonEncryptionPublicInput {                 encrypted_data:
// encrypted_data.clone().into_inner(),                 k_hash_value:
// pvde_zkp.public_input().k_hash_value().clone(),             };
//             let is_valid = verify_poseidon_encryption(
//                 &poseidon_encryption_zkp_param,
//                 &poseidon_encryption_verify_key,
//                 &poseidon_encryption_public_input,
//                 &pvde_zkp.encryption_proof().clone().into_inner(),
//             );

//             if !is_valid {
//                 return Err(Error::PvdeZkpInvalid);
//             }
//             info!("Done verify_poseidon_encryption: {:?}", is_valid);
//         }
//         false => {}
//     }

//     // TODO(jaemin): generalize
//     let eth_encrypt_data: EthPlainData =
// serde_json::from_str(&decrypted_data).unwrap();

//     let rollup_transaction = match open_data {
//         OpenData::Eth(open_data) =>
// open_data.convert_to_rollup_transaction(&eth_encrypt_data),         _ =>
// unreachable!(),     };

//     let eth_raw_transaction =
// EthRawTransaction::from(to_raw_tx(rollup_transaction));
//     let raw_transaction = RawTransaction::from(eth_raw_transaction);

//     Ok(raw_transaction)
// }

#[allow(clippy::too_many_arguments)]
pub async fn issue_order_commitment(
    context: Arc<AppState>,
    platform: Platform,
    rollup_id: String,
    order_commitment_type: OrderCommitmentType,
    transaction_hash: RawTransactionHash,
    rollup_block_height: u64,
    transaction_order: u64,
    order_hash: OrderHash,
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
                previous_order_hash: order_hash,
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

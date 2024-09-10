use std::{collections::HashMap, fs, path::PathBuf, sync::Arc};

use pvde::{
    encryption::poseidon_encryption_zkp::{
        export_proving_key as export_poseidon_encryption_proving_key,
        export_verifying_key as export_poseidon_encryption_verifying_key,
        export_zkp_param as export_poseidon_encryption_zkp_param,
        import_proving_key as import_poseidon_encryption_proving_key,
        import_verifying_key as import_poseidon_encryption_verifying_key,
        import_zkp_param as import_poseidon_encryption_zkp_param,
        setup as setup_poseidon_encryption,
    },
    time_lock_puzzle::{
        export_time_lock_puzzle_param, import_time_lock_puzzle_param,
        key_validation_zkp::{
            export_proving_key as export_key_validation_proving_key,
            export_verifying_key as export_key_validation_verifying_key,
            export_zkp_param as export_key_validation_zkp_param,
            import_proving_key as import_key_validation_proving_key,
            import_verifying_key as import_key_validation_verifying_key,
            import_zkp_param as import_key_validation_zkp_param, setup as setup_key_validation,
        },
        setup as setup_time_lock_puzzle_param,
    },
};
use radius_sequencer_sdk::{json_rpc::RpcServer, kvstore::KvStore as Database};
use sequencer::{
    cli::{Cli, Commands},
    client::liveness::seeder::SeederClient,
    error::{self, Error},
    rpc::{cluster, external, internal},
    state::AppState,
    types::*,
};
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt().init();

    let mut cli = Cli::init();

    match cli.command {
        Commands::Init { ref config_path } => ConfigPath::init(config_path)?,
        Commands::Start {
            ref mut config_option,
        } => {
            // Load the configuration from the path
            let config = Config::load(config_option)?;

            tracing::info!(
                "Successfully loaded the configuration file at {:?}.",
                config.path(),
            );

            // Initialize the database
            Database::new(config.database_path())
                .map_err(error::Error::Database)?
                .init();
            tracing::info!(
                "Successfully initialized the database at {:?}.",
                config.database_path(),
            );

            // Initialize sequencing info
            // let sequencing_info_list = SequencingInfoListModel::get_or_default();
            // sequencing_info_list.iter().for_each(|sequencing_info| {
            //     info!(
            //         "platform: {:?}, sequencing_function_type: {:?}, service_type: {:?}",
            //         sequencing_info.platform(),
            //         sequencing_info.sequencing_function_type(),
            //         sequencing_info.service_type()
            //     );
            // });

            // Initialize seeder client
            let seeder_rpc_url = config.seeder_rpc_url();
            let seeder_client = SeederClient::new(seeder_rpc_url)?;
            tracing::info!(
                "Successfully initialized seeder client {:?}.",
                seeder_rpc_url,
            );

            // get or init rollup id list model
            // let rollup_id_list_model = match RollupIdListModel::get() {
            //     Ok(rollup_id_list_model) => rollup_id_list_model,
            //     Err(err) => {
            //         if err.is_none_type() {
            //             let new_rollup_id_list_model = RollupIdListModel::default();
            //             new_rollup_id_list_model.put()?;
            //             new_rollup_id_list_model
            //         } else {
            //             return Err(err.into());
            //         }
            //     }
            // };

            // let rollup_id_list = rollup_id_list_model.rollup_id_list();

            // let rollup_states = rollup_id_list
            //     .iter()
            //     .map(|rollup_id| -> Result<(RollupId, RollupState), Error> {
            //         let rollup_model = RollupModel::get(rollup_id)?;
            //         let cluster_id = rollup_model.cluster_id().clone();
            //         let rollup_block_height = RollupMetadataModel::get(rollup_id)?
            //             .rollup_metadata()
            //             .block_height();

            //         Ok((
            //             rollup_id.clone(),
            //             RollupState::new(cluster_id, rollup_block_height),
            //         ))
            //     })
            //     .collect::<Result<HashMap<RollupId, RollupState>, Error>>()?;

            let pvde_params = if let Some(ref path) = config_option.path {
                // Initialize the time lock puzzle parameters.
                Some(init_time_lock_puzzle_param(path, config.is_using_zkp())?)
            } else {
                None
            };

            // get or init sequencing infos, can be loaded after restarting
            // let sequencing_info_model = match SequencingInfoModel::get() {
            //     Ok(sequencing_info_model) => sequencing_info_model,
            //     Err(err) => {
            //         if err.is_none_type() {
            //             SequencingInfoModel::new(HashMap::new())
            //         } else {
            //             return Err(err.into());
            //         }
            //     }
            // };
            // let sequencing_infos = sequencing_info_model.sequencing_infos();

            // Initialize an application-wide state instance
            let app_state = AppState::new(config, seeder_client);

            // Add listener for each sequencing info
            // sequencing_infos.iter().for_each(
            //     |(sequencing_info_key, sequencing_info)| {
            //         info!(
            //             "platform: {:?}, sequencing_function_type: {:?}, service_type: {:?}",
            //             sequencing_info_key.platform(), sequencing_info_key.sequencing_function_type(), sequencing_info_key.service_type()
            //         );

            //         match sequencing_info_key.platform() {
            //             Platform::Local => {
            //                 // TODO:
            //                 info!("Init local platform (TODO)");
            //             }
            //             Platform::Ethereum => match sequencing_info_key.sequencing_function_type() {
            //                 sequencer::types::SequencingFunctionType::Liveness => {
            //                     match sequencing_info_key.service_type() {
            //                         ServiceType::Radius => {
            //                             info!(
            //                                 "Init radius liveness - provider_websocket_url: {:?}",
            //                                 sequencing_info.provider_websocket_url
            //                             );

            //                             let sync_info = SyncInfo::new(
            //                                 sequencing_info.clone(),
            //                                 Arc::new(app_state.clone()),
            //                             );
            //                             radius_liveness_event_listener::init(
            //                                 Arc::new(sync_info),
            //                             );
            //                         }
            //                         _ => {
            //                             // TODO:
            //                             info!(
            //                                 "Init other liveness (TODO) - provider_websocket_url: {:?}",
            //                                 sequencing_info.provider_websocket_url
            //                             );
            //                         }
            //                     }
            //                 }
            //                 sequencer::types::SequencingFunctionType::Validation => {}
            //             },
            //         }
            //     },
            // );

            // Initialize the internal RPC server
            initialize_internal_rpc_server(&app_state).await?;

            // Initialize the cluster RPC server
            initialize_cluster_rpc_server(&app_state).await?;

            // Initialize clusters
            initialize_clusters(&app_state).await?;

            // SKDE
            // for rollup_id in rollup_id_list.iter() {
            //     let rollup_model = RollupModel::get(rollup_id).unwrap();
            //     let cluster_id = rollup_model.cluster_id().clone();

            //     let cluster = app_state.get_cluster(&cluster_id).await.unwrap();

            //     // TODO: only skde
            //     init_single_key_generator(rollup_id.clone(), cluster);
            // }

            // Initialize the external RPC server.
            let server_handle = initialize_external_rpc_server(&app_state).await?;

            server_handle.await.unwrap();
        }
    }

    Ok(())
}

async fn initialize_clusters(app_state: &AppState) -> Result<(), Error> {
    // let config = app_state.config();
    // let seeder_client = app_state.seeder_client();
    // let signing_key = config.signing_key();

    // let address = config.address();

    // // The cluster rpc url is the rpc url of the sequencer
    // let cluster_rpc_url = config.cluster_rpc_url().to_string();

    // // Register sequencer rpc url (with cluster rpc url) to seeder
    // match seeder_client.get_rpc_url(&address).await {
    //     Ok(rpc_url) => {
    //         if rpc_url != cluster_rpc_url {
    //             // TODO: Check
    //             seeder_client
    //                 .register_rpc_url(address.clone(), cluster_rpc_url.to_string())
    //                 .await?;
    //         }
    //     }
    //     Err(_) => {
    //         seeder_client
    //             .register_rpc_url(address.clone(), cluster_rpc_url.to_string())
    //             .await?;
    //     }
    // }

    // // get or init sequencing infos, can be loaded after restarting
    // let sequencing_info_model = match SequencingInfoModel::get() {
    //     Ok(sequencing_info_model) => sequencing_info_model,
    //     Err(err) => {
    //         if err.is_none_type() {
    //             SequencingInfoModel::default()
    //         } else {
    //             return Err(err.into());
    //         }
    //     }
    // };

    // for (sequencing_info_key, sequencing_info) in sequencing_info_model.sequencing_infos().iter() {
    //     info!(
    //         "platform: {:?}, sequencing_function_type: {:?}, service_type: {:?}",
    //         sequencing_info_key.platform(),
    //         sequencing_info_key.sequencing_function_type(),
    //         sequencing_info_key.service_type()
    //     );

    //     // Get all cluster ids for each sequencing info
    //     let cluster_id_list_model = match ClusterIdListModel::get(
    //         sequencing_info_key.platform(),
    //         sequencing_info_key.sequencing_function_type(),
    //         sequencing_info_key.service_type(),
    //     ) {
    //         Ok(cluster_id_list_model) => cluster_id_list_model,
    //         Err(err) => {
    //             if err.is_none_type() {
    //                 continue;
    //             } else {
    //                 return Err(err.into());
    //             }
    //         }
    //     };

    //     // Initialize each cluster
    //     for cluster_id in cluster_id_list_model.cluster_id_list().iter() {
    //         info!("initialize_cluster: {:?}", cluster_id);

    //         match sequencing_info_key.sequencing_function_type() {
    //             SequencingFunctionType::Liveness => {
    //                 let cluster = initialize_liveness_cluster(
    //                     &SigningKey::from(signing_key.clone()),
    //                     &seeder_client,
    //                     sequencing_info,
    //                     cluster_id,
    //                 )
    //                 .await?;

    //                 app_state.set_cluster(cluster);
    //             }
    //             SequencingFunctionType::Validation => {
    //                 // TODO:
    //             }
    //         }
    //     }
    // }
    Ok(())
}

async fn initialize_internal_rpc_server(app_state: &AppState) -> Result<(), Error> {
    // let internal_rpc_url = app_state.config().internal_rpc_url().to_string();

    // // Initialize the internal RPC server.
    // let internal_rpc_server = RpcServer::new(app_state)
    //     // Todo: implement
    //     .init(app_state.config().internal_rpc_url().to_string())
    //     .await?;

    // tracing::info!(
    //     "Successfully started the internal RPC server: {}",
    //     internal_rpc_url
    // );

    // tokio::spawn(async move {
    //     internal_rpc_server.stopped().await;
    // });

    Ok(())
}

async fn initialize_cluster_rpc_server(app_state: &AppState) -> Result<(), Error> {
    // let cluster_rpc_url = app_state.config().cluster_rpc_url().to_string();

    // let sequencer_rpc_server = RpcServer::new(app_state)
    //     .init(cluster_rpc_url.clone())
    //     .await?;

    // tracing::info!(
    //     "Successfully started the cluster RPC server: {}",
    //     cluster_rpc_url
    // );

    // tokio::spawn(async move {
    //     sequencer_rpc_server.stopped().await;
    // });

    Ok(())
}

async fn initialize_external_rpc_server(app_state: &AppState) -> Result<JoinHandle<()>, Error> {
    let sequencer_rpc_url = app_state.config().sequencer_rpc_url().to_string();

    // Initialize the external RPC server.
    // let external_rpc_server = RpcServer::new(app_state.clone())
    //     .init(sequencer_rpc_url.clone())
    //     .await?;

    // tracing::info!(
    //     "Successfully started the sequencer RPC server: {}",
    //     sequencer_rpc_url
    // );

    let server_handle = tokio::spawn(async move {
        // external_rpc_server.stopped().await;
    });

    Ok(server_handle)
}

pub fn init_time_lock_puzzle_param(
    config_path: &PathBuf,
    is_using_zkp: bool,
) -> Result<PvdeParams, Error> {
    let time_lock_puzzle_param_path = config_path
        .join("time_lock_puzzle_param.json")
        .to_str()
        .unwrap()
        .to_string();

    let time_lock_puzzle_param = if fs::metadata(&time_lock_puzzle_param_path).is_ok() {
        import_time_lock_puzzle_param(&time_lock_puzzle_param_path)
    } else {
        let time_lock_puzzle_param = setup_time_lock_puzzle_param(2048);
        export_time_lock_puzzle_param(&time_lock_puzzle_param_path, time_lock_puzzle_param.clone());
        time_lock_puzzle_param
    };

    let mut pvde_params = PvdeParams::default();
    pvde_params.update_time_lock_puzzle_param(time_lock_puzzle_param);

    if is_using_zkp {
        let key_validation_param_file_path = config_path
            .join("key_validation_zkp_param.data")
            .to_str()
            .unwrap()
            .to_string();
        let key_validation_proving_key_file_path = config_path
            .join("key_validation_proving_key.data")
            .to_str()
            .unwrap()
            .to_string();
        let key_validation_verifying_key_file_path = config_path
            .join("key_validation_verifying_key.data")
            .to_str()
            .unwrap()
            .to_string();

        let (key_validation_zkp_param, key_validation_verifying_key, key_validation_proving_key) =
            if fs::metadata(&key_validation_param_file_path).is_ok() {
                (
                    import_key_validation_zkp_param(&key_validation_param_file_path),
                    import_key_validation_verifying_key(&key_validation_verifying_key_file_path),
                    import_key_validation_proving_key(&key_validation_proving_key_file_path),
                )
            } else {
                let setup_results = setup_key_validation(13);
                export_key_validation_zkp_param(
                    &key_validation_param_file_path,
                    setup_results.0.clone(),
                );
                export_key_validation_verifying_key(
                    &key_validation_verifying_key_file_path,
                    setup_results.1.clone(),
                );
                export_key_validation_proving_key(
                    &key_validation_proving_key_file_path,
                    setup_results.2.clone(),
                );
                setup_results
            };

        pvde_params.update_key_validation_zkp_param(key_validation_zkp_param);
        pvde_params.update_key_validation_proving_key(key_validation_proving_key);
        pvde_params.update_key_validation_verifying_key(key_validation_verifying_key);

        let poseidon_encryption_param_file_path = config_path
            .join("poseidon_encryption_param.json")
            .to_str()
            .unwrap()
            .to_string();
        let poseidon_encryption_proving_key_file_path = config_path
            .join("poseidon_encryption_proving_key.data")
            .to_str()
            .unwrap()
            .to_string();
        let poseidon_encryption_verifying_key_file_path = config_path
            .join("poseidon_encryption_verifying_key.data")
            .to_str()
            .unwrap()
            .to_string();

        let (
            poseidon_encryption_zkp_param,
            poseidon_encryption_verifying_key,
            poseidon_encryption_proving_key,
        ) = if fs::metadata(&poseidon_encryption_param_file_path).is_ok() {
            (
                import_poseidon_encryption_zkp_param(&poseidon_encryption_param_file_path),
                import_poseidon_encryption_verifying_key(
                    &poseidon_encryption_verifying_key_file_path,
                ),
                import_poseidon_encryption_proving_key(&poseidon_encryption_proving_key_file_path),
            )
        } else {
            let setup_results = setup_poseidon_encryption(13);
            export_poseidon_encryption_zkp_param(
                &poseidon_encryption_param_file_path,
                setup_results.0.clone(),
            );
            export_poseidon_encryption_verifying_key(
                &poseidon_encryption_verifying_key_file_path,
                setup_results.1.clone(),
            );
            export_poseidon_encryption_proving_key(
                &poseidon_encryption_proving_key_file_path,
                setup_results.2.clone(),
            );
            setup_results
        };

        pvde_params.update_poseidon_encryption_zkp_param(poseidon_encryption_zkp_param);
        pvde_params.update_poseidon_encryption_proving_key(poseidon_encryption_proving_key);
        pvde_params.update_poseidon_encryption_verifying_key(poseidon_encryption_verifying_key);
    }

    Ok(pvde_params)
}
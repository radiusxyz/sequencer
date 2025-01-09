use clap::{Parser, Subcommand};
use radius_sdk::{
    json_rpc::server::RpcServer,
    kvstore::{CachedKvStore, KvStoreBuilder},
    util::{get_resource_limit, set_resource_limit, ResourceType},
};
use sequencer::{
    client::{
        liveness::{
            self, distributed_key_generation::DistributedKeyGenerationClient, seeder::SeederClient,
        },
        validation,
    },
    error::{self, Error},
    logger::Logger,
    profiler::Profiler,
    rpc::{cluster, external, internal},
    state::AppState,
    types::*,
};
pub use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;

#[derive(Debug, Deserialize, Parser, Serialize)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    pub fn init() -> Self {
        Cli::parse()
    }
}

#[derive(Subcommand, Debug, Deserialize, Serialize)]
pub enum Commands {
    /// Initializes a node
    Init {
        #[clap(flatten)]
        config_path: Box<ConfigPath>,
    },

    /// Starts the node
    Start {
        #[clap(flatten)]
        config_option: Box<ConfigOption>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut cli = Cli::init();
    match cli.command {
        Commands::Init { ref config_path } => ConfigPath::init(config_path)?,
        Commands::Start {
            ref mut config_option,
        } => {
            let rlimit = get_resource_limit(ResourceType::RLIMIT_NOFILE)?;
            set_resource_limit(ResourceType::RLIMIT_NOFILE, rlimit.hard_limit)?;

            // Initialize the profiler.
            let profiler = Profiler::init("http://127.0.0.1:4040", "sequencer", 100)?;

            // Load the configuration from the path
            let config = Config::load(config_option)?;

            // Initialize the logger.
            Logger::new(config.log_path())
                .map_err(error::Error::LoggerError)?
                .init();

            tracing::info!(
                "Successfully loaded the configuration file at {:?}.",
                config.path,
            );

            // Initialize the database
            KvStoreBuilder::default()
                .set_default_lock_timeout(5000)
                .set_txn_lock_timeout(5000)
                .build(config.database_path())
                .map_err(error::Error::Database)?
                .init();

            // Database::open(config.database_path())
            //     .map_err(error::Error::Database)?
            //     .init();
            tracing::info!(
                "Successfully initialized the database at {:?}.",
                config.database_path(),
            );

            // Initialize seeder client
            let seeder_rpc_url = &config.seeder_rpc_url;
            let seeder_client = SeederClient::new(seeder_rpc_url)?;
            tracing::info!(
                "Successfully initialized seeder client {:?}.",
                seeder_rpc_url,
            );

            // Initialize distributed key generation client
            let distributed_key_generation_rpc_url = &config.distributed_key_generation_rpc_url;
            let distributed_key_generation_client =
                DistributedKeyGenerationClient::new(distributed_key_generation_rpc_url)?;

            let signers = CachedKvStore::default();
            let liveness_clients = CachedKvStore::default();
            let validation_clients = CachedKvStore::default();

            let skde_params = distributed_key_generation_client
                .get_skde_params()
                .await?
                .skde_params;

            // Initialize an application-wide state instance
            let app_state: AppState = AppState::new(
                config,
                seeder_client,
                distributed_key_generation_client,
                signers,
                liveness_clients,
                validation_clients,
                skde_params,
                profiler,
            );

            // Initialize liveness clients
            let sequencing_info_list =
                SequencingInfoList::get_or(SequencingInfoList::default).map_err(Error::Database)?;
            for (platform, service_provider) in sequencing_info_list.iter() {
                tracing::info!(
                    "Initialize sequencing info - platform: {:?}, service_provider: {:?}",
                    platform,
                    service_provider
                );
                let sequencing_info_payload =
                    SequencingInfoPayload::get(*platform, *service_provider)
                        .map_err(Error::Database)?;

                match sequencing_info_payload {
                    SequencingInfoPayload::Ethereum(liveness_info) => {
                        liveness::radius::LivenessClient::initialize(
                            app_state.clone(),
                            *platform,
                            *service_provider,
                            liveness_info,
                        );
                    }
                    SequencingInfoPayload::Local(_payload) => {
                        // liveness::local::LivenessClient::new()?;
                        todo!("Implement 'LivenessClient' for local sequencing.");
                    }
                }
            }

            // Initialize validation clients
            let validation_service_providers =
                ValidationServiceProviders::get_or(ValidationServiceProviders::default)
                    .map_err(Error::Database)?;
            for (platform, validation_service_provider) in validation_service_providers.iter() {
                let validation_info = ValidationInfo::get(*platform, *validation_service_provider)
                    .map_err(Error::Database)?;

                match validation_info {
                    ValidationInfo::EigenLayer(eigen_layer_validation_info) => {
                        validation::eigenlayer::ValidationClient::initialize(
                            app_state.clone(),
                            *platform,
                            *validation_service_provider,
                            eigen_layer_validation_info,
                        );
                    }
                    ValidationInfo::Symbiotic(symbiotic_validation_info) => {
                        validation::symbiotic::ValidationClient::initialize(
                            app_state.clone(),
                            *platform,
                            *validation_service_provider,
                            symbiotic_validation_info,
                        );
                    }
                }
            }

            // Initialize the internal RPC server
            initialize_internal_rpc_server(&app_state).await?;

            // Initialize the cluster RPC server
            initialize_cluster_rpc_server(&app_state).await?;

            // Initialize the external RPC server.
            let server_handle = initialize_external_rpc_server(&app_state).await?;

            server_handle.await.unwrap();
        }
    }

    Ok(())
}

async fn initialize_internal_rpc_server(context: &AppState) -> Result<(), Error> {
    let internal_rpc_url = context.config().internal_rpc_url.to_string();

    // Initialize the internal RPC server.
    let internal_rpc_server = RpcServer::new(context.clone())
        .register_rpc_method(
            internal::AddSequencingInfo::METHOD_NAME,
            internal::AddSequencingInfo::handler,
        )?
        .register_rpc_method(
            internal::AddValidationInfo::METHOD_NAME,
            internal::AddValidationInfo::handler,
        )?
        .register_rpc_method(
            internal::AddCluster::METHOD_NAME,
            internal::AddCluster::handler,
        )?
        .register_rpc_method(
            internal::GetCluster::METHOD_NAME,
            internal::GetCluster::handler,
        )?
        .register_rpc_method(
            internal::GetClusterIdList::METHOD_NAME,
            internal::GetClusterIdList::handler,
        )?
        .register_rpc_method(
            internal::GetSequencingInfos::METHOD_NAME,
            internal::GetSequencingInfos::handler,
        )?
        .register_rpc_method(
            internal::GetSequencingInfo::METHOD_NAME,
            internal::GetSequencingInfo::handler,
        )?
        .init(internal_rpc_url.clone())
        .await?;

    tracing::info!(
        "Successfully started the internal RPC server: {}",
        internal_rpc_url
    );

    tokio::spawn(async move {
        internal_rpc_server.stopped().await;
    });

    Ok(())
}

async fn initialize_cluster_rpc_server(context: &AppState) -> Result<(), Error> {
    let cluster_rpc_url = anywhere(&context.config().cluster_port()?);

    let sequencer_rpc_server = RpcServer::new(context.clone())
        .register_rpc_method(
            cluster::SyncEncryptedTransaction::METHOD_NAME,
            cluster::SyncEncryptedTransaction::handler,
        )?
        .register_rpc_method(
            cluster::SyncRawTransaction::METHOD_NAME,
            cluster::SyncRawTransaction::handler,
        )?
        .register_rpc_method(
            cluster::FinalizeBlock::METHOD_NAME,
            cluster::FinalizeBlock::handler,
        )?
        .register_rpc_method(cluster::SyncBlock::METHOD_NAME, cluster::SyncBlock::handler)?
        .register_rpc_method(
            external::GetRawTransactionList::METHOD_NAME,
            external::GetRawTransactionList::handler,
        )?
        .init(cluster_rpc_url.clone())
        .await?;

    tracing::info!(
        "Successfully started the cluster RPC server: {}",
        cluster_rpc_url
    );

    tokio::spawn(async move {
        sequencer_rpc_server.stopped().await;
    });

    Ok(())
}

async fn initialize_external_rpc_server(context: &AppState) -> Result<JoinHandle<()>, Error> {
    let external_rpc_url = anywhere(&context.config().external_port()?);

    // Initialize the external RPC server.
    let external_rpc_server = RpcServer::new(context.clone())
        .register_rpc_method(
            external::SendEncryptedTransaction::METHOD_NAME,
            external::SendEncryptedTransaction::handler,
        )?
        .register_rpc_method(
            external::GetEncryptedTransactionWithTransactionHash::METHOD_NAME,
            external::GetEncryptedTransactionWithTransactionHash::handler,
        )?
        .register_rpc_method(
            external::GetEncryptedTransactionWithOrderCommitment::METHOD_NAME,
            external::GetEncryptedTransactionWithOrderCommitment::handler,
        )?
        .register_rpc_method(
            external::GetRawTransactionWithTransactionHash::METHOD_NAME,
            external::GetRawTransactionWithTransactionHash::handler,
        )?
        .register_rpc_method(
            external::GetRawTransactionWithOrderCommitment::METHOD_NAME,
            external::GetRawTransactionWithOrderCommitment::handler,
        )?
        .register_rpc_method(
            external::GetOrderCommitment::METHOD_NAME,
            external::GetOrderCommitment::handler,
        )?
        .register_rpc_method(
            external::SendRawTransaction::METHOD_NAME,
            external::SendRawTransaction::handler,
        )?
        .register_rpc_method(
            external::GetRawTransactionList::METHOD_NAME,
            external::GetRawTransactionList::handler,
        )?
        .register_rpc_method(
            external::GetRollup::METHOD_NAME,
            external::GetRollup::handler,
        )?
        .register_rpc_method(
            external::GetRollupMetadata::METHOD_NAME,
            external::GetRollupMetadata::handler,
        )?
        .register_rpc_method(external::GetBlock::METHOD_NAME, external::GetBlock::handler)?
        .init(external_rpc_url.clone())
        .await?;

    tracing::info!(
        "Successfully started the sequencer external RPC server: {}",
        external_rpc_url
    );

    let server_handle = tokio::spawn(async move {
        external_rpc_server.stopped().await;
    });

    Ok(server_handle)
}

pub fn anywhere(port: &str) -> String {
    format!("0.0.0.0:{}", port)
}

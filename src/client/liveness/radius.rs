use std::{str::FromStr, sync::Arc};

use radius_sdk::{
    liveness_radius::{
        publisher::Publisher,
        subscriber::Subscriber,
        types::{rpc::types::Header, Events, Liveness::LivenessEvents},
    },
    signature::{Address, PrivateKeySigner},
};
use tokio::time::{sleep, Duration};

use crate::{client::liveness::seeder::SeederClient, error::Error, state::AppState, types::*};

pub struct LivenessClient {
    inner: Arc<LivenessClientInner>,
}

struct LivenessClientInner {
    platform: Platform,
    service_provider: ServiceProvider,
    publisher: Publisher,
    subscriber: Subscriber,
    seeder: SeederClient,
}

impl Clone for LivenessClient {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl LivenessClient {
    pub fn platform(&self) -> Platform {
        self.inner.platform
    }

    pub fn service_provider(&self) -> ServiceProvider {
        self.inner.service_provider
    }

    pub fn publisher(&self) -> &Publisher {
        &self.inner.publisher
    }

    pub fn subscriber(&self) -> &Subscriber {
        &self.inner.subscriber
    }

    pub fn seeder(&self) -> &SeederClient {
        &self.inner.seeder
    }

    pub fn new(
        platform: Platform,
        service_provider: ServiceProvider,
        liveness_info: LivenessRadius,
        signing_key: impl AsRef<str>,
        seeder: SeederClient,
    ) -> Result<Self, Error> {
        let publisher = Publisher::new(
            liveness_info.liveness_rpc_url,
            signing_key,
            &liveness_info.contract_address,
        )
        .map_err(|error| Error::CreateLivenessClient(error.into()))?;

        let subscriber = Subscriber::new(
            liveness_info.liveness_websocket_url,
            liveness_info.contract_address,
        )
        .map_err(|error| Error::CreateLivenessClient(error.into()))?;

        let inner = LivenessClientInner {
            platform,
            service_provider,
            publisher,
            subscriber,
            seeder,
        };

        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    pub fn initialize(
        context: AppState,
        platform: Platform,
        service_provider: ServiceProvider,
        liveness_info: LivenessRadius,
    ) {
        let handle = tokio::spawn({
            let context = context.clone();
            let liveness_info = liveness_info.clone();

            async move {
                let signing_key = context.config().signing_key();
                let signer = PrivateKeySigner::from_str(platform.into(), signing_key).unwrap();
                context.add_signer(platform, signer).await.unwrap();

                let liveness_client = Self::new(
                    platform,
                    service_provider,
                    liveness_info,
                    signing_key,
                    context.seeder_client().clone(),
                )
                .unwrap();

                context
                    .add_liveness_client(platform, service_provider, liveness_client.clone())
                    .await
                    .unwrap();

                tracing::info!(
                    "Initializing the liveness event listener for {:?}, {:?}..",
                    platform,
                    service_provider
                );
                liveness_client
                    .subscriber()
                    .initialize_event_handler(callback, liveness_client.clone())
                    .await
                    .unwrap();
            }
        });

        tokio::spawn(async move {
            if handle.await.is_err() {
                tracing::warn!(
                    "Reconnecting the liveness event listener for {:?}, {:?}..",
                    platform,
                    service_provider
                );
                sleep(Duration::from_secs(5)).await;
                Self::initialize(context, platform, service_provider, liveness_info);
            }
        });
    }
}

async fn callback(events: Events, liveness_client: LivenessClient) {
    match events {
        Events::Block(block) => on_new_block(block, liveness_client).await,
        Events::LivenessEvents(liveness_event, log) => match liveness_event {
            LivenessEvents::RegisterSequencer(register_sequencer) => {
                let cluster_id = register_sequencer.clusterId;
                let platform_block_height = log.block_number.unwrap();
                let sequencer_index: usize = register_sequencer.index.try_into().unwrap();
                let sequencer_address = register_sequencer.sequencerAddress.to_string();
                let sequencer_rpc_info = liveness_client
                    .seeder()
                    .get_sequencer_rpc_url(sequencer_address.to_string())
                    .await
                    .unwrap()
                    .sequencer_rpc_url;

                let mut liveness_event_list = LivenessEventList::get_mut_or(
                    &cluster_id,
                    platform_block_height,
                    LivenessEventList::default,
                )
                .unwrap();
                liveness_event_list.push((sequencer_index, sequencer_rpc_info));
                liveness_event_list.update().unwrap();
            }
            LivenessEvents::DeregisterSequencer(deregister_sequencer) => {
                let cluster_id = deregister_sequencer.clusterId;
                let platform_block_height = log.block_number.unwrap();
                let sequencer_address = deregister_sequencer.sequencerAddress.to_string();

                let mut liveness_event_list = LivenessEventList::get_mut_or(
                    &cluster_id,
                    platform_block_height,
                    LivenessEventList::default,
                )
                .unwrap();
                liveness_event_list.push(sequencer_address);
                liveness_event_list.update().unwrap();
            }
            _others => {}
        },
    }
}

async fn on_new_block(block: Header, liveness_client: LivenessClient) {
    let platform_block_height = block.number;
    let previous_block_height = platform_block_height.wrapping_sub(1);

    let cluster_id_list = ClusterIdList::get_or(
        liveness_client.platform(),
        liveness_client.service_provider(),
        ClusterIdList::default,
    )
    .unwrap();

    for cluster_id in cluster_id_list.iter() {
        // Build `Cluster` for the current block height from the previous block height.
        match Cluster::get(
            liveness_client.platform(),
            liveness_client.service_provider(),
            cluster_id,
            previous_block_height,
        ) {
            Ok(mut cluster) => {
                let liveness_event_list = LivenessEventList::get_or(
                    cluster_id,
                    previous_block_height,
                    LivenessEventList::default,
                )
                .unwrap();
                for event in liveness_event_list.iter() {
                    match event {
                        LivenessEventType::RegisterSequencer((
                            sequencer_index,
                            sequencer_rpc_info,
                        )) => {
                            cluster
                                .register_sequencer(*sequencer_index, sequencer_rpc_info.clone());
                        }
                        LivenessEventType::DeregisterSequencer(sequencer_address) => {
                            cluster.deregister_sequencer(sequencer_address);
                        }
                    }
                }

                cluster.adjust_my_index(&liveness_client.publisher().address().to_string());
                cluster
                    .put(
                        liveness_client.platform(),
                        liveness_client.service_provider(),
                        cluster_id,
                        platform_block_height,
                    )
                    .unwrap();
            }
            Err(error) => {
                if error.is_none_type() {
                    let mut my_index: Option<usize> = None;

                    // Get the sequencer address list for the cluster ID.
                    let sequencer_address_list: Vec<String> = liveness_client
                        .publisher()
                        .get_sequencer_list(cluster_id, platform_block_height)
                        .await
                        .unwrap()
                        .into_iter()
                        .enumerate()
                        .map(|(index, address)| {
                            if address == liveness_client.publisher().address() {
                                my_index = Some(index);
                            }

                            address.to_string()
                        })
                        .collect();

                    // Get the sequencer RPC URL list for the cluster.
                    let sequencer_rpc_url_list = liveness_client
                        .seeder()
                        .get_sequencer_rpc_url_list(sequencer_address_list.clone())
                        .await
                        .unwrap()
                        .sequencer_rpc_url_list;

                    // Get the rollup info list
                    let rollup_info_list = liveness_client
                        .publisher()
                        .get_rollup_info_list(cluster_id, platform_block_height)
                        .await
                        .unwrap();

                    let rollup_id_list = rollup_info_list
                        .iter()
                        .map(|rollup_info| rollup_info.rollupId.clone())
                        .collect();

                    // Update the rollup info to database
                    for rollup_info in rollup_info_list {
                        match Rollup::get_mut(&rollup_info.rollupId) {
                            Ok(mut rollup) => {
                                let new_executor_address_list = rollup_info
                                    .executorAddresses
                                    .into_iter()
                                    .map(|address| {
                                        Address::from_str(
                                            Platform::from_str(
                                                &rollup_info.validationInfo.platform,
                                            )
                                            .unwrap()
                                            .into(),
                                            &address.to_string(),
                                        )
                                        .unwrap()
                                    })
                                    .collect::<Vec<Address>>();

                                rollup.set_executor_address_list(new_executor_address_list);
                                rollup.update().unwrap();
                            }
                            Err(error) => {
                                if error.is_none_type() {
                                    let order_commitment_type = OrderCommitmentType::from_str(
                                        &rollup_info.orderCommitmentType,
                                    )
                                    .unwrap();
                                    let rollup_type =
                                        RollupType::from_str(&rollup_info.rollupType).unwrap();
                                    let platform =
                                        Platform::from_str(&rollup_info.validationInfo.platform)
                                            .unwrap();
                                    let validation_service_provider =
                                        ValidationServiceProvider::from_str(
                                            &rollup_info.validationInfo.serviceProvider,
                                        )
                                        .unwrap();

                                    let validation_service_manager = Address::from_str(
                                        platform.into(),
                                        &rollup_info
                                            .validationInfo
                                            .validationServiceManager
                                            .to_string(),
                                    )
                                    .unwrap();

                                    let validation_info = ValidationInfo::new(
                                        platform,
                                        validation_service_provider,
                                        validation_service_manager,
                                    );
                                    let executor_address_list = rollup_info
                                        .executorAddresses
                                        .into_iter()
                                        .map(|address| {
                                            Address::from_str(
                                                Platform::from_str(
                                                    &rollup_info.validationInfo.platform,
                                                )
                                                .unwrap()
                                                .into(),
                                                &address.to_string(),
                                            )
                                            .unwrap()
                                        })
                                        .collect::<Vec<Address>>();

                                    let rollup = Rollup::new(
                                        rollup_info.rollupId.clone(),
                                        rollup_type,
                                        EncryptedTransactionType::Skde,
                                        Address::from_str(
                                            Platform::from_str(
                                                &rollup_info.validationInfo.platform,
                                            )
                                            .unwrap()
                                            .into(),
                                            &rollup_info.owner.to_string(),
                                        )
                                        .unwrap(),
                                        validation_info,
                                        order_commitment_type,
                                        executor_address_list,
                                        cluster_id.to_owned(),
                                        liveness_client.platform(),
                                        liveness_client.service_provider(),
                                    );

                                    Rollup::put(&rollup, rollup.rollup_id()).unwrap();
                                }
                            }
                        }
                    }

                    let block_margin = liveness_client
                        .publisher()
                        .get_block_margin()
                        .await
                        .unwrap();

                    let cluster = Cluster::new(
                        sequencer_rpc_url_list,
                        rollup_id_list,
                        my_index.unwrap(),
                        block_margin.try_into().unwrap(),
                    );

                    Cluster::put_and_update_with_margin(
                        &cluster,
                        liveness_client.platform(),
                        liveness_client.service_provider(),
                        cluster_id,
                        platform_block_height,
                    )
                    .unwrap();
                } else {
                    panic!("{:?}", error);
                }
            }
        }
    }
}

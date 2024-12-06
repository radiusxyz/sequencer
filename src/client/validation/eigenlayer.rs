use std::sync::Arc;

use radius_sdk::validation_eigenlayer::{
    publisher::Publisher,
    subscriber::Subscriber,
    types::{Avs, Bytes, IValidationServiceManager},
};
use tokio::time::{sleep, Duration};

use crate::{error::Error, types::*};

pub struct ValidationClient {
    inner: Arc<ValidationClientInner>,
}

struct ValidationClientInner {
    platform: Platform,
    validation_service_provider: ValidationServiceProvider,
    publisher: Publisher,
    subscriber: Subscriber,
}

unsafe impl Send for ValidationClient {}

unsafe impl Sync for ValidationClient {}

impl Clone for ValidationClient {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl ValidationClient {
    pub fn new(
        platform: Platform,
        validation_service_provider: ValidationServiceProvider,
        validation_info: ValidationEigenLayer,
        signing_key: impl AsRef<str>,
    ) -> Result<Self, Error> {
        let publisher = Publisher::new(
            validation_info.validation_rpc_url,
            signing_key,
            validation_info.delegation_manager_contract_address,
            validation_info.avs_directory_contract_address,
            validation_info.stake_registry_contract_address,
            validation_info.avs_contract_address.clone(),
        )
        .map_err(|error| Error::InitializeValidationClient(error.into()))?;

        let subscriber = Subscriber::new(
            validation_info.validation_websocket_url,
            validation_info.avs_contract_address,
        )
        .map_err(|error| Error::InitializeValidationClient(error.into()))?;

        let inner = ValidationClientInner {
            platform,
            validation_service_provider,
            publisher,
            subscriber,
        };

        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    pub fn platform(&self) -> Platform {
        self.inner.platform
    }

    pub fn validation_service_provider(&self) -> ValidationServiceProvider {
        self.inner.validation_service_provider
    }

    pub fn publisher(&self) -> &Publisher {
        &self.inner.publisher
    }

    pub fn subscriber(&self) -> &Subscriber {
        &self.inner.subscriber
    }

    pub fn initialize_event_listener(&self) {
        tracing::info!(
            "Initializing EigenLayer validation event listener for {:?}, {:?}..",
            self.platform(),
            self.validation_service_provider()
        );

        let handle = tokio::spawn({
            let validation_client = self.clone();

            async move {
                validation_client
                    .subscriber()
                    .initialize_event_handler(callback, validation_client.clone())
                    .await
                    .unwrap();
            }
        });

        tokio::spawn({
            let validation_client = self.clone();

            async move {
                if handle.await.is_err() {
                    tracing::warn!("Reconnecting EigenLayer validation event listener..");
                    sleep(Duration::from_secs(5)).await;
                    validation_client.initialize_event_listener();
                }
            }
        });
    }
}

async fn callback(event: Avs::NewTaskCreated, context: ValidationClient) {
    let rollup = Rollup::get(&event.rollupId).ok();
    if let Some(rollup) = rollup {
        let block = Block::get(rollup.rollup_id(), event.task.blockNumber).unwrap();

        if block.block_creator_address != context.publisher().address() {
            let task = IValidationServiceManager::Task {
                commitment: Bytes::from_iter(&[0u8; 32]),
                blockNumber: 0,
                rollupId: rollup.rollup_id().to_owned(),
                clusterId: rollup.cluster_id().to_owned(),
                taskCreatedBlock: event.taskCreatedBlock,
            };

            let transaction_hash = context
                .publisher()
                .respond_to_task(task, event.taskIndex, Bytes::from_iter(&[0_u8; 64]))
                .await
                .unwrap();
            tracing::info!("[EigenLayer] respond_to_task: {:?}", transaction_hash);
        }
    }
}

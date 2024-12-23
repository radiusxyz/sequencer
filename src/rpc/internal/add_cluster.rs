use radius_sdk::signature::PrivateKeySigner;

use crate::rpc::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AddCluster {
    pub platform: Platform,
    pub service_provider: ServiceProvider,
    pub cluster_id: String,
}

impl AddCluster {
    pub const METHOD_NAME: &'static str = "add_cluster";

    pub async fn handler(parameter: RpcParameter, context: Arc<AppState>) -> Result<(), RpcError> {
        let parameter = parameter.parse::<Self>()?;

        tracing::info!(
            "Add cluster - platform: {:?}, service provider: {:?}, cluster id: {:?}",
            parameter.platform,
            parameter.service_provider,
            parameter.cluster_id
        );

        let seeder_client = context.seeder_client();
        match parameter.platform {
            Platform::Ethereum => {
                let signing_key = &context.config().signing_key;
                let signer = PrivateKeySigner::from_str(parameter.platform.into(), signing_key)?;

                seeder_client
                    .register_sequencer(
                        parameter.platform,
                        parameter.service_provider,
                        &parameter.cluster_id,
                        &context.config().external_rpc_url,
                        &context.config().cluster_rpc_url,
                        &signer,
                    )
                    .await?;

                let mut cluster_id_list = ClusterIdList::get_mut_or(
                    parameter.platform,
                    parameter.service_provider,
                    ClusterIdList::default,
                )?;
                cluster_id_list.insert(&parameter.cluster_id);
                cluster_id_list.update()?;
            }
            Platform::Local => unimplemented!("Local client needs to be implemented."),
        }

        Ok(())
    }
}

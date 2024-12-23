use radius_sdk::signature::PrivateKeySigner;

use crate::rpc::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Deregister {
    pub platform: Platform,
    pub service_provider: ServiceProvider,
    pub cluster_id: String,
}

impl Deregister {
    pub const METHOD_NAME: &'static str = stringify!(Deregister);

    pub async fn handler(parameter: RpcParameter, context: Arc<AppState>) -> Result<(), RpcError> {
        let parameter = parameter.parse::<Self>()?;

        tracing::info!(
            "Deregister - platform: {:?}, service provider: {:?}, cluster id: {:?}",
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
                    .deregister_sequencer(
                        parameter.platform,
                        parameter.service_provider,
                        &parameter.cluster_id,
                        &signer,
                    )
                    .await?;

                let mut cluster_id_list =
                    ClusterIdList::get_mut(parameter.platform, parameter.service_provider)?;
                cluster_id_list.remove(&parameter.cluster_id);
                cluster_id_list.update()?;
            }
            Platform::Local => unimplemented!("Local client needs to be implemented."),
        }

        Ok(())
    }
}

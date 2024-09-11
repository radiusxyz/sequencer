use crate::rpc::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetRollup {
    platform: Platform,
    service_provider: ServiceProvider,

    cluster_id: String,
    rollup_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetRollupResponse {
    rollup: Rollup,
}

impl GetRollup {
    pub const METHOD_NAME: &'static str = "get_rollup";

    pub async fn handler(
        parameter: RpcParameter,
        _context: Arc<AppState>,
    ) -> Result<GetRollupResponse, RpcError> {
        let parameter = parameter.parse::<Self>()?;

        let rollup = RollupModel::get(&parameter.rollup_id)?;

        Ok(GetRollupResponse { rollup })
    }
}

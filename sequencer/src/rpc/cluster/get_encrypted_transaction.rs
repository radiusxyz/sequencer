use crate::rpc::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetEncryptedTransaction {
    pub rollup_block_number: u64,
    pub transaction_order: u64,
}

impl GetEncryptedTransaction {
    pub const METHOD_NAME: &'static str = stringify!(GetEncryptedTransaction);

    pub async fn handler(
        parameter: RpcParameter,
        _context: Arc<AppState>,
    ) -> Result<UserEncryptedTransaction, RpcError> {
        let parameter = parameter.parse::<Self>()?;

        UserEncryptedTransaction::get(parameter.rollup_block_number, parameter.transaction_order)
            .map_err(|error| error.into())
    }
}

use std::sync::Arc;

use radius_sdk::json_rpc::client::{Id, RpcClient};
use serde::{Deserialize, Serialize};

pub struct DistributedKeyGenerationClient {
    inner: Arc<DistributedKeyGenerationClientInner>,
}

struct DistributedKeyGenerationClientInner {
    rpc_url: String,
    rpc_client: RpcClient,
}

impl Clone for DistributedKeyGenerationClient {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl DistributedKeyGenerationClient {
    pub fn new(rpc_url: impl AsRef<str>) -> Result<Self, DistributedKeyGenerationClientError> {
        let inner = DistributedKeyGenerationClientInner {
            rpc_url: rpc_url.as_ref().to_owned(),
            rpc_client: RpcClient::new()
                .map_err(DistributedKeyGenerationClientError::Initialize)?,
        };

        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    pub async fn get_decryption_key(
        &self,
        key_id: u64,
    ) -> Result<GetDecryptionKeyResponse, DistributedKeyGenerationClientError> {
        let parameter = GetDecryptionKey { key_id };

        self.inner
            .rpc_client
            .request(
                &self.inner.rpc_url,
                GetDecryptionKey::METHOD_NAME,
                &parameter,
                Id::Null,
            )
            .await
            .map_err(DistributedKeyGenerationClientError::GetDecryptionKey)
    }

    pub async fn get_skde_params(
        &self,
    ) -> Result<GetSkdeParamsResponse, DistributedKeyGenerationClientError> {
        let parameter = GetSkdeParams {};

        self.inner
            .rpc_client
            .request(
                &self.inner.rpc_url,
                GetSkdeParams::METHOD_NAME,
                &parameter,
                Id::Null,
            )
            .await
            .map_err(DistributedKeyGenerationClientError::GetSkdeParams)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetDecryptionKey {
    pub key_id: u64,
}

impl GetDecryptionKey {
    pub const METHOD_NAME: &'static str = "get_decryption_key";
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetDecryptionKeyResponse {
    pub decryption_key: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetSkdeParams {}

impl GetSkdeParams {
    pub const METHOD_NAME: &'static str = "get_skde_params";
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetSkdeParamsResponse {
    pub skde_params: skde::delay_encryption::SkdeParams,
}

#[derive(Debug)]
pub enum DistributedKeyGenerationClientError {
    Initialize(radius_sdk::json_rpc::client::RpcClientError),
    GetEncryptionKey(radius_sdk::json_rpc::client::RpcClientError),
    GetDecryptionKey(radius_sdk::json_rpc::client::RpcClientError),
    GetLatestEncryptionKey(radius_sdk::json_rpc::client::RpcClientError),
    GetSkdeParams(radius_sdk::json_rpc::client::RpcClientError),
}

impl std::fmt::Display for DistributedKeyGenerationClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for DistributedKeyGenerationClientError {}

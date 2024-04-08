use std::{fmt::Debug, sync::Arc};

use sequencer_core::{
    async_trait::async_trait,
    error::Error,
    jsonrpsee::core::traits::ToRpcParams,
    serde::{de::DeserializeOwned, ser::Serialize},
};
use sequencer_database::Database;

/// Defines the necessary traits for a type to be used as an RPC parameter.
///
/// # Examples
/// ```
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Example {}
///
/// #[async_trait]
/// impl RpcMethod for Example {
///     type Response = String;
///
///     fn method_name() ->  &'static str {
///         ""
///     }
///
///     async fn handler(self) -> Result<Self::Output, Error> {
///         Ok(String::from(""))
///     }
/// }
/// ```
#[async_trait]
pub trait RpcMethod: Clone + Debug + DeserializeOwned + Serialize + ToRpcParams + Send {
    /// The type of the output that the RPC method returns.
    type Response: Clone + Debug + DeserializeOwned + Serialize + Send;

    /// Provides the method name for the RPC parameter type.
    ///
    /// # Returns
    ///
    /// A static string slice representing the RPC method's name.
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Assuming a struct MyRpc that implements RpcParameter.
    /// let method = MyRpc::method_name();
    /// println!("RPC Method Name: {}", method);
    /// ```
    fn method_name() -> &'static str;

    /// A handler function to process the RPC request.
    ///
    /// # Returns
    ///
    /// A result containing the desired output on success, or an error on failure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Assuming a struct MyRpc that implements RpcParameter and an instance `rpc`.
    /// let result = rpc.handler().await;
    /// match result {
    ///     Ok(output) => println!("Output: {:?}", output),
    ///     Err(err) => eprintln!("Error: {:?}", err),
    /// }
    /// ```
    async fn handler(self, state: Arc<Database>) -> Result<Self::Response, Error>;
}
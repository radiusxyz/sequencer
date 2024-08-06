mod cluster_id_list;
mod liveness_cluster;
mod validation_cluster;

pub use cluster_id_list::*;
pub use liveness_cluster::*;
use serde::{Deserialize, Serialize};
pub use validation_cluster::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClusterModel {
    Liveness(LivenessClusterModel),
    Validation(ValidationClusterModel),
}
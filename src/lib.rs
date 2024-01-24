use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("SerializationError: {0}")]
    SerializationError(#[source] serde_json::Error),

    #[error("Kube Error: {0}")]
    KubeError(#[source] kube::Error),

    #[error("MachineError Error: {0}")]
    // NB: awkward type because finalizer::Error embeds the reconciler error (which is this)
    // so boxing this error to break cycles
    MachineError(#[source] Box<kube::runtime::finalizer::Error<Error>>),

    #[error("Owner Machine not found")]
    MachineNotFound,

    #[error("Please associate this machine with a cluster using the label {CLUSTER_NAME_LABEL}: <name of cluster>")]
    ClusterNotFound,

    #[error("IP family unknown: {0}")]
    IPFamilyUnknown(#[source] std::net::AddrParseError),

    #[error("IllegalDocument")]
    IllegalDocument,
}
pub type Result<T, E = Error> = std::result::Result<T, E>;

impl Error {
    pub fn metric_label(&self) -> String {
        format!("{self:?}").to_lowercase()
    }
}

const CLUSTER_NAME_LABEL: &str = "cluster.x-k8s.io/cluster-name";

pub mod api;
/// Expose all controller components used by main
pub mod controller;
pub mod controllers;
pub mod docker;
pub use crate::controller::*;

use k8s_openapi::NamespaceResourceScope;
use kube::{runtime::reflector::ObjectRef, Api, Client};
use kube_core::{Resource, ResourceExt};
use serde::de::DeserializeOwned;
use std::fmt::Debug;

use crate::api::{cluster::Cluster, machines::Machine};

pub fn to_machine<M: Resource<DynamicType = ()>>(machine: Machine) -> Option<ObjectRef<M>> {
    match (
        machine.spec.infrastructure_ref.kind,
        machine.spec.infrastructure_ref.api_version,
        machine.spec.infrastructure_ref.name,
        machine.spec.infrastructure_ref.namespace,
    ) {
        (Some(kind), Some(api_version), Some(name), namespace)
            if kind == M::kind(&()) && api_version == M::api_version(&()) =>
        {
            Some(ObjectRef::new(name.as_str()).within(namespace.unwrap_or_default().as_str()))
        }
        _ => None,
    }
}

pub fn cluster_to_machine<M>(client: Client, cluster: Cluster) -> Option<ObjectRef<M>>
where
    M: Resource<DynamicType = (), Scope = NamespaceResourceScope>,
    M: Clone + Sync + Send + Debug,
    M: DeserializeOwned,
{
    let api: Api<M> = Api::namespaced(client, cluster.namespace().unwrap_or_default().as_str());

    api.list(&Default::default());
    None
    // match (
    //     machine.spec.infrastructure_ref.kind,
    //     machine.spec.infrastructure_ref.api_version,
    //     machine.spec.infrastructure_ref.name,
    //     machine.spec.infrastructure_ref.namespace,
    // ) {
    //     (Some(kind), Some(api_version), Some(name), namespace)
    //         if kind == M::kind(&()) && api_version == M::api_version(&()) =>
    //     {
    //         Some(ObjectRef::new(name.as_str()).within(namespace.unwrap_or_default().as_str()))
    //     }
    //     _ => None,
    // }
}

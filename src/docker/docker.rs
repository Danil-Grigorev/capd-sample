use std::sync::Arc;

use kube_core::ResourceExt;

use crate::{api::cluster::Cluster, controllers::docker::ClusterIPFamily, Context, Error, Result};

// Node can be thought of as a logical component of Kubernetes.
// A node is either a control plane node, a worker node, or a load balancer node.
struct Node {
    name: String,
    cluster_role: String,
    internal_ip: String,
    image: String,
    status: String,
    // Commander:   *ContainerCmder
}

// Machine implement a service for managing the docker containers hosting a kubernetes nodes.
struct Machine {
    cluster: String,
    machine: String,
    pod_ips: ClusterIPFamily,
    service_ips: ClusterIPFamily,
    // container   *types.Node
    // nodeCreator nodeCreator
}

impl Machine {
    fn new(ctx: Arc<Context>, cluster: Cluster, machine: String) -> Result<Machine> {
        // filters := container.FilterBuilder{}
        // filters.AddKeyNameValue(filterLabel, clusterLabelKey, cluster)
        // filters.AddKeyValue(filterName, fmt.Sprintf("^%s$", machineContainerName(cluster, machine)))
        // for key, val := range filterLabels {
        //     filters.AddKeyNameValue(filterLabel, key, val)
        // }

        // let _container = getContainer(ctx, filters)?;

        Ok(Machine {
            machine,
            // container,
            cluster: cluster.name_any(),
            pod_ips: cluster.get_pod_ip_family()?,
            service_ips: cluster.get_services_ip_family()?,
            // nodeCreator: &Manager{},
        })
    }
}

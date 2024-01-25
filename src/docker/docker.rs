use std::{collections::HashMap, sync::Arc};

use docker_api::{
    models::ContainerSummary,
    opts::{ContainerFilter, ContainerListOpts},
};
use kube_core::ResourceExt;

use crate::{
    api::cluster::Cluster, controllers::docker::ClusterIPFamily, Context, Error, Result,
    CLUSTER_LABEL_KEY,
};

// Node can be thought of as a logical component of Kubernetes.
// A node is either a control plane node, a worker node, or a load balancer node.
struct Node {
    name: Option<String>,
    cluster_role: Option<String>,
    internal_ip: Option<String>,
    image: Option<String>,
    status: Option<String>,
    // Commander:   *ContainerCmder
}

// Machine implement a service for managing the docker containers hosting a kubernetes nodes.
pub struct Machine {
    cluster: String,
    machine: String,
    pod_ips: ClusterIPFamily,
    service_ips: ClusterIPFamily,
    container: Node,
    // nodeCreator nodeCreator
}

impl Machine {
    pub async fn new(
        ctx: Arc<Context>,
        cluster: Cluster,
        machine: String,
        labels: HashMap<String, String>,
    ) -> Result<Machine> {
        let filters = vec![
            ContainerFilter::Label(CLUSTER_LABEL_KEY.to_string(), cluster.name_any()),
            ContainerFilter::LabelKey(format!("^{}-{machine}$", cluster.name_any())),
        ]
        .into_iter()
        .chain(
            labels
                .into_iter()
                .map(|(k, v)| ContainerFilter::Label(k, v)),
        );

        // let _container = getContainer(ctx, filters)?;

        Ok(Machine {
            machine,
            container: Machine::get_container(filters.collect()).await?,
            cluster: cluster.name_any(),
            pod_ips: cluster.get_pod_ip_family()?,
            service_ips: cluster.get_services_ip_family()?,
            // nodeCreator: &Manager{},
        })
    }

    pub async fn get_container(filters: Vec<ContainerFilter>) -> Result<Node> {
        let container_list = Machine::list_containers(filters).await?;
        let container = container_list
            .first()
            .ok_or(Error::ContainerLookupError)?
            .clone();
        let names = container.names.unwrap_or_default();

        Ok(Node {
            name: match names.first() {
                Some(name) => Some(name.clone()),
                None => None,
            },
            cluster_role: None,
            internal_ip: None,
            image: container.image,
            status: container.status,
        })
    }

    pub async fn list_containers(filters: Vec<ContainerFilter>) -> Result<Vec<ContainerSummary>> {
        let api = docker_api::Docker::new("unix:///var/run/docker.sock")
            .map_err(Error::ContainerError)?;
        api.containers()
            .list(&ContainerListOpts::builder().filter(filters).build())
            .await
            .map_err(Error::ContainerError)
    }
}

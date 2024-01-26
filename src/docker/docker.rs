use std::collections::HashMap;

use docker_api::{
    models::ContainerSummary,
    opts::{ContainerCreateOpts, ContainerCreateOptsBuilder, ContainerFilter, ContainerListOpts},
    Container,
};
use kube_core::ResourceExt;
use tracing::info;

use crate::{
    api::{cluster::Cluster, machines::Machine},
    controllers::docker::ClusterIPFamily,
    Error, Result, CLUSTER_LABEL_KEY,
};

// Node can be thought of as a logical component of Kubernetes.
// A node is either a control plane node, a worker node, or a load balancer node.
pub struct Node {
    name: Option<String>,
    cluster_role: Option<String>,
    internal_ip: Option<String>,
    image: Option<String>,
    status: Option<String>,
    // Commander:   *ContainerCmder
}

// Association implement a service for managing the docker containers hosting a kubernetes nodes.
pub struct Association {
    pub cluster: Cluster,
    pub machine: Machine,
    // pod_ips: ClusterIPFamily,
    // service_ips: ClusterIPFamily,
    container: Node,
    // nodeCreator nodeCreator
}

impl Association {
    pub async fn new(
        cluster: Cluster,
        machine: Machine,
        labels: HashMap<String, String>,
    ) -> Result<Association> {
        let filters = vec![
            ContainerFilter::Label(CLUSTER_LABEL_KEY.to_string(), cluster.name_any()),
            // ContainerFilter::LabelKey(format!("^{}-{}$", cluster.name_any(), machine.name_any())),
        ]
        .into_iter();
        // .chain(
        //     labels
        //         .into_iter()
        //         .map(|(k, v)| ContainerFilter::Label(k, v)),
        // );

        Ok(Association {
            machine,
            container: Association::get_container(filters.collect()).await?,
            cluster: cluster.clone(),
            // pod_ips: cluster.get_pod_ip_family()?,
            // service_ips: cluster.get_services_ip_family()?,
            // nodeCreator: &Manager{},
        })
    }

    pub async fn get_container(filters: Vec<ContainerFilter>) -> Result<Node> {
        let container_list = Association::list_containers(filters).await?;
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

    pub async fn create_container(&self) -> Result<Container> {
        let image = "kindest/node:v1.26.3";
        let api = docker_api::Docker::new("unix:///var/run/docker.sock")
            .map_err(Error::ContainerError)?;
        let container = api.containers()
            .create(&ContainerCreateOptsBuilder::default().name("test").image(image).build())
            .await
            .map_err(Error::ContainerCreateError)?;
        info!("{container:?}");
        Ok(container)
    }
}

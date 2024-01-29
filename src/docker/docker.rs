use std::{collections::BTreeMap, net::TcpListener};

use docker_api::{models::ContainerSummary, opts::ContainerFilter};
use kube_core::ResourceExt;

use crate::{
    api::{cluster::Cluster, machines::Machine},
    Error, Result, CLUSTER_LABEL_KEY, DEFAULT_DOCKER_SOCKET, DEFAULT_IMAGE, DEFAULT_VERSION,
    HASH_LABEL_KEY, MACHINE_CONTROL_PLANE_LABEL, NODE_ROLE_LABEL_KEY,
};

use super::container::{
    docker::Runtime,
    interface::{Mount, PortMapping, RunContainerInput},
};

// Node can be thought of as a logical component of Kubernetes.
// A node is either a control plane node, a worker node, or a load balancer node.
#[derive(Debug)]
pub struct Node {
    name: String,
    image: String,
    status: Option<String>,
    // Commander:   *ContainerCmder
}

pub enum MachineRole {
    Worker(Association),
    ControlPlane(Association),
}

impl MachineRole {
    fn from_machine(association: Association) -> Self {
        match association
            .machine
            .labels()
            .contains_key(MACHINE_CONTROL_PLANE_LABEL)
        {
            true => MachineRole::ControlPlane(association),
            false => MachineRole::Worker(association),
        }
    }

    fn role_label(&self) -> (String, String) {
        match self {
            MachineRole::Worker(_) => (NODE_ROLE_LABEL_KEY.to_string(), "worker".to_string()),
            MachineRole::ControlPlane(_) => {
                (NODE_ROLE_LABEL_KEY.to_string(), "control-plane".to_string())
            }
        }
    }

    fn cluster_label(&self) -> (String, String) {
        match self {
            MachineRole::Worker(a) | MachineRole::ControlPlane(a) => {
                (CLUSTER_LABEL_KEY.to_string(), a.cluster.name_any())
            }
        }
    }

    fn hash_label(&self) -> (String, String) {
        (
            HASH_LABEL_KEY.to_string(),
            self.base_create().get_hash().to_string(),
        )
    }

    fn get_filters(&self) -> Vec<ContainerFilter> {
        self.get_labels()
            .iter()
            .map(|(k, v)| ContainerFilter::Label(k.clone(), v.clone()))
            .collect()
    }

    fn get_labels(&self) -> BTreeMap<String, String> {
        BTreeMap::from([self.role_label(), self.hash_label(), self.cluster_label()])
    }

    async fn create_machine(&self) -> Result<()> {
        match self {
            MachineRole::Worker(association) | MachineRole::ControlPlane(association) => {
                if association
                    .get_container(self.get_filters())
                    .await?
                    .is_some()
                {
                    return Ok(());
                }
            }
        };

        match self {
            MachineRole::Worker(association) | MachineRole::ControlPlane(association) => {
                let container = association
                    .runtime
                    .create_container(self.create_input()?)
                    .await?;

                association
                    .runtime
                    .exec(container.id().to_string(), vec!["crictl", "ps"])
                    .await
            }
        }
    }

    async fn delete_machine(&self) -> Result<String> {
        match self {
            MachineRole::Worker(association) | MachineRole::ControlPlane(association) => {
                let container_id = match association
                    .runtime
                    .list_containers(self.get_filters())
                    .await?
                    .into_iter()
                    .find(|c| {
                        c.names.clone().is_some_and(|names| {
                            names.iter().any(|n| {
                                n.contains(format!("/{}", &association.container_name()).as_str())
                            })
                        })
                    }) {
                    Some(ContainerSummary { id: Some(id), .. }) => id.clone(),
                    _ => return Ok(Default::default()),
                };

                association.runtime.delete_container(container_id).await
            }
        }
    }

    fn create_input(&self) -> Result<RunContainerInput> {
        Ok(match self {
            MachineRole::Worker(association) => RunContainerInput {
                image: association.get_image(),
                labels: self.get_labels(),
                ..self.base_create()
            },
            MachineRole::ControlPlane(association) => {
                let socket = TcpListener::bind("127.0.0.1:0").unwrap();
                let addr = socket.local_addr().map_err(Error::PortLookupError)?;
                RunContainerInput {
                    image: association.get_image(),
                    labels: self.get_labels(),
                    port_mappings: vec![PortMapping {
                        container_port: 6443,
                        host_port: addr.port(),
                        protocol: "tcp".to_string(),
                    }],
                    ..self.base_create()
                }
            }
        })
    }

    fn base_create(&self) -> RunContainerInput {
        match self {
            Self::Worker(association) | Self::ControlPlane(association) => RunContainerInput {
                mounts: association.generate_mount_info(),
                name: association.container_name(),
                network: "kind".to_string(),
                // ip_family: Default::default(),
                ..Default::default()
            },
        }
    }
}

// Association implement a service for managing the docker containers hosting a kubernetes nodes.
#[derive(Clone)]
pub struct Association {
    pub runtime: Runtime,
    pub cluster: Cluster,
    pub machine: Machine,
    pub custom_image: Option<String>,
    // pod_ips: ClusterIPFamily,
    // service_ips: ClusterIPFamily,
}

impl Association {
    pub async fn new(
        cluster: Cluster,
        machine: Machine,
        custom_image: Option<String>,
    ) -> Result<Association> {
        Ok(Association {
            machine,
            custom_image,
            runtime: Runtime::new()?,
            cluster: cluster.clone(),
            // pod_ips: cluster.get_pod_ip_family()?,
            // service_ips: cluster.get_services_ip_family()?,
        })
    }

    pub async fn get_container(&self, filters: Vec<ContainerFilter>) -> Result<Option<Node>> {
        let container = match self
            .runtime
            .list_containers(filters)
            .await?
            .into_iter()
            .find(|c| {
                c.names.clone().is_some_and(|names| {
                    names
                        .iter()
                        .any(|n| n.contains(format!("/{}", &self.container_name()).as_str()))
                })
            }) {
            Some(container) => container.clone(),
            None => return Ok(None),
        };

        Ok(
            match (container.names.unwrap_or_default().first(), container.image) {
                (Some(name), Some(image)) => Some(Node {
                    image,
                    name: name.to_string(),
                    status: container.status,
                }),
                _ => None,
            },
        )
    }

    pub async fn create(self) -> Result<()> {
        MachineRole::from_machine(self).create_machine().await
    }

    pub async fn delete(self) -> Result<String> {
        MachineRole::from_machine(self).delete_machine().await
    }

    fn get_image(&self) -> String {
        if let Some(image) = self.custom_image.clone() {
            return image;
        }

        match self.machine.spec.version.clone() {
            Some(version) => match version.as_bytes() {
                [b'v', ..] => format!("{DEFAULT_IMAGE}:{}", version.to_string()),
                _ => format!("{DEFAULT_IMAGE}:v{version}"),
            },
            None => format!("{DEFAULT_IMAGE}:{DEFAULT_VERSION}"),
        }
    }

    fn container_name(&self) -> String {
        let (cluster, machine) = (self.cluster.name_any(), self.machine.name_any());

        match machine.starts_with(cluster.as_str()) {
            true => machine,
            false => format!("{cluster}-{machine}"),
        }
    }

    fn generate_mount_info(&self) -> Vec<Mount> {
        vec![
            // some k8s things want to read /lib/modules
            Mount {
                source: Some("/lib/modules".into()),
                target: "/lib/modules".into(),
                read_only: true,
            },
            Mount {
                source: Some(DEFAULT_DOCKER_SOCKET.into()),
                target: DEFAULT_DOCKER_SOCKET.into(),
                read_only: false,
            },
            // runtime persistent storage
            // this ensures that E.G. pods, logs etc. are not on the container
            // filesystem, which is not only better for performance
            Mount {
                source: None,
                target: "/var".to_string(),
                read_only: false,
            },
            // tmpfs
            Mount {
                source: None,
                target: "/tmp".to_string(),
                read_only: false,
            },
            Mount {
                source: None,
                target: "/run".to_string(),
                read_only: false,
            },
        ]
    }
}

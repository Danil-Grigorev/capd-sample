use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::Arc,
    time::Duration,
};

use kube::{runtime::controller::Action, Api};
use kube_core::{Resource, ResourceExt};
use tracing::{error, info};

use crate::{
    api::{
        cluster::{
            self, Cluster, ClusterClusterNetwork, ClusterClusterNetworkPods,
            ClusterClusterNetworkServices, ClusterInfrastructureRef, ClusterStatus,
        },
        dockerclusters::DockerCluster,
        dockermachines::{DockerMachine, DockerMachineStatus},
        machines::Machine,
    },
    docker::{
        container::interface::ClusterIPFamily,
        docker::{self, Association, MachineRole, Node},
    },
    Context, Error, Result, CLUSTER_NAME_LABEL,
};

impl Machine {
    pub async fn get_cluster(&self, ctx: Arc<Context>) -> Result<Cluster> {
        let name = self
            .labels()
            .iter()
            .find(|(label, _)| label.as_str() == CLUSTER_NAME_LABEL)
            .map(|(_, cluster_name)| cluster_name)
            .ok_or(Error::ClusterNotFound)?;

        let clusters: Api<Cluster> = Api::namespaced(
            ctx.client.clone(),
            self.namespace().unwrap_or_default().as_str(),
        );

        clusters.get(name.as_str()).await.map_err(Error::KubeError)
    }
}

impl Cluster {
    pub async fn get_cluster(&self, ctx: Arc<Context>) -> Option<DockerCluster> {
        let clusters: Api<DockerCluster> = Api::namespaced(
            ctx.client.clone(),
            self.namespace().unwrap_or_default().as_str(),
        );

        match self.spec.infrastructure_ref.clone() {
            Some(ClusterInfrastructureRef {
                name: Some(cluster_ref),
                ..
            }) => clusters.get(cluster_ref.as_str()).await.ok(),
            _ => {
                info!("Cluster infrastructureRef is not available yet");
                None
            }
        }
    }

    pub fn get_pod_ip_family(&self) -> Result<ClusterIPFamily> {
        match self.spec.cluster_network.clone() {
            Some(ClusterClusterNetwork {
                pods:
                    Some(ClusterClusterNetworkPods {
                        cidr_blocks: pod_cidr_blocks,
                    }),
                ..
            }) => ClusterIPFamily::new(pod_cidr_blocks),
            _ => ClusterIPFamily::new(vec![]),
        }
    }

    pub fn get_services_ip_family(&self) -> Result<ClusterIPFamily> {
        match self.spec.cluster_network.clone() {
            Some(ClusterClusterNetwork {
                services:
                    Some(ClusterClusterNetworkServices {
                        cidr_blocks: service_cidr_blocks,
                    }),
                ..
            }) => ClusterIPFamily::new(service_cidr_blocks),
            _ => ClusterIPFamily::new(vec![]),
        }
    }
}

impl DockerMachine {
    pub async fn get_owner(&self, ctx: Arc<Context>) -> Result<Option<Machine>> {
        let name = self
            .owner_references()
            .iter()
            .find(|m| m.kind == Machine::kind(&()))
            .map(|m| m.name.clone())
            .ok_or(Error::MachineNotFound)?;

        let machines: Api<Machine> = Api::namespaced(
            ctx.client.clone(),
            self.namespace().unwrap_or_default().as_str(),
        );

        match machines.get(name.as_str()).await {
            Ok(machine) => Ok(Some(machine)),
            Err(kube::Error::Api(ae)) if ae.code == 404 => Ok(None),
            Err(e) => Err(Error::KubeError(e)),
        }
    }

    pub async fn set_machine_address(&self, ctx: Arc<Context>) -> Result<()> {
        let association = self.get_association(ctx).await?;

        let container = association
            .clone()
            .get_container(MachineRole::from_machine(association).get_filters())
            .await?;

        let network = match container {
            Some(Node {
                network: Some(network),
                ..
            }) => network,
            _ => return Ok(()),
        };

        // TODO: something with status.
        // dockerMachine.Status.Addresses = []clusterv1.MachineAddress{{
        // 	Type:    clusterv1.MachineHostName,
        // 	Address: externalMachine.ContainerName()},
        // }

        // for _, addr := range machineAddresses {
        // 	dockerMachine.Status.Addresses = append(dockerMachine.Status.Addresses,
        // 		clusterv1.MachineAddress{
        // 			Type:    clusterv1.MachineInternalIP,
        // 			Address: addr,
        // 		},
        // 		clusterv1.MachineAddress{
        // 			Type:    clusterv1.MachineExternalIP,
        // 			Address: addr,
        // 		})
        // }

        Ok(())
    }

    pub async fn get_association(&self, ctx: Arc<Context>) -> Result<Association> {
        let machine = match self.get_owner(ctx.clone()).await? {
            Some(machine) => machine,
            None => {
                info!("Waiting for Machine Controller to set OwnerRef on DockerMachine");
                return Err(Error::MachineNotFound);
            }
        };

        let cluster = machine.get_cluster(ctx.clone()).await?;
        let docker_cluster = match cluster.get_cluster(ctx.clone()).await {
            Some(cluster) => cluster,
            None => {
                info!("DockerCluster is not available yet");
                return Err(Error::DockerClusterNotFound);
            }
        };

        Association::new(cluster, machine, self.spec.custom_image.clone()).await
    }

    pub async fn reconcile(&self, ctx: Arc<Context>) -> Result<Action> {
        match self.get_association(ctx.clone()).await {
            Ok(association) => self.reconcile_normal(ctx, association).await,
            Err(Error::ClusterNotFound) | Err(Error::MachineNotFound) => {
                Ok(Action::requeue(Duration::from_secs(5 * 60)))
            }
            Err(e) => Err(e),
        }
    }

    pub async fn reconcile_normal(
        &self,
        ctx: Arc<Context>,
        association: Association,
    ) -> Result<Action> {
        let mut status = self.status.clone().unwrap_or_default();

        // Check if the infrastructure is ready, otherwise return and wait for the cluster object to be updated
        match association.cluster {
            Cluster {
                status:
                    Some(ClusterStatus {
                        infrastructure_ready: Some(true),
                        ..
                    }),
                ..
            } => (),
            _ => {
                info!("Waiting for DockerCluster Controller to create cluster infrastructure");
                // 	conditions.MarkFalse(dockerMachine, infrav1.ContainerProvisionedCondition, infrav1.WaitingForClusterInfrastructureReason, clusterv1.ConditionSeverityInfo, "")
                return Ok(Action::requeue(Duration::from_secs(5 * 60)));
            }
        }

        match self.spec.provider_id.clone() {
            Some(_) => {
                status.ready = Some(true);
                self.set_machine_address(ctx.clone()).await?
            }
            None => (),
        }
        
        match association.prepare_bootstrap(ctx).await {
            Ok(_) => (),
            Err(Error::BootstrapSecretNotReady) => {
                return Ok(Action::requeue(Duration::from_secs(5 * 60)))
            }
            Err(e) => return Err(e),
        };
        
        association.create().await?;

        // Preload?..

        // if the machine is a control plane update the load balancer configuration
        // we should only do this once, as reconfiguration more or less ensures
        // node ref setting fails
        match status.load_balancer_configured {
            Some(false) | None => (), //todo!("Update configuration"); status.load_balancer_configured = Some(true),
            Some(true) => (),
        };

        info!("done");

        Ok(Action::requeue(Duration::from_secs(5 * 60)))
    }

    pub async fn reconcile_delete(&self, association: Association) -> Result<Action> {
        association.delete().await?;

        Ok(Action::requeue(Duration::from_secs(5 * 60)))
    }

    pub async fn cleanup(&self, ctx: Arc<Context>) -> Result<Action> {
        match self.get_association(ctx).await {
            Ok(association) => self.reconcile_delete(association).await,
            Err(Error::ClusterNotFound) | Err(Error::MachineNotFound) => {
                Ok(Action::requeue(Duration::from_secs(5 * 60)))
            }
            Err(e) => Err(e),
        }
    }
}

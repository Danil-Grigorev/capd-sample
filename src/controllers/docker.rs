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
        docker::{self, Association},
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

    pub async fn reconcile(&self, ctx: Arc<Context>) -> Result<Action> {
        let machine = match self.get_owner(ctx.clone()).await? {
            Some(machine) => machine,
            None => {
                info!("Waiting for Machine Controller to set OwnerRef on DockerMachine");
                return Ok(Action::requeue(Duration::from_secs(5 * 60)));
            }
        };

        let cluster = machine.get_cluster(ctx.clone()).await?;
        let docker_cluster = match cluster.get_cluster(ctx.clone()).await {
            Some(cluster) => cluster,
            None => {
                info!("DockerCluster is not available yet");
                return Ok(Action::requeue(Duration::from_secs(5 * 60)));
            }
        };

        self.reconcile_normal(
            Association::new(cluster, machine, self.spec.custom_image.clone()).await?,
        )
        .await?;

        Ok(Action::requeue(Duration::from_secs(5 * 60)))
    }

    pub async fn reconcile_normal(&self, association: Association) -> Result<Action> {
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
            Some(_) => status.ready = Some(true),
            None => (),
        }

        match association.machine.spec.bootstrap.data_secret_name {
            None => {
                // // if !util.IsControlPlaneMachine(machine) && !conditions.IsTrue(cluster, clusterv1.ControlPlaneInitializedCondition) {
                //     log.Info("Waiting for the control plane to be initialized")
                //     conditions.MarkFalse(dockerMachine, infrav1.ContainerProvisionedCondition, clusterv1.WaitingForControlPlaneAvailableReason, clusterv1.ConditionSeverityInfo, "")
                //     return ctrl.Result{}, nil
                // }

                // log.Info("Waiting for the Bootstrap provider controller to set bootstrap data")
                // conditions.MarkFalse(dockerMachine, infrav1.ContainerProvisionedCondition, infrav1.WaitingForBootstrapDataReason, clusterv1.ConditionSeverityInfo, "")
            }
            _ => (),
        }

        association.create().await?;
        info!("done");

        Ok(Action::requeue(Duration::from_secs(5 * 60)))
    }

    pub async fn cleanup(&self, _ctx: Arc<Context>) -> Result<Action> {
        Ok(Action::requeue(Duration::from_secs(5 * 60)))
    }
}

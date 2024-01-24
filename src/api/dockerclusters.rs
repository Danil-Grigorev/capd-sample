// WARNING: generated by kopium - manual changes will be overwritten
// kopium command: kopium -Af -
// kopium version: 0.16.5

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

/// DockerClusterSpec defines the desired state of DockerCluster.
#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[kube(group = "infrastructure.cluster.x-k8s.io", version = "v1beta1", kind = "DockerCluster", plural = "dockerclusters")]
#[kube(namespaced)]
#[kube(status = "DockerClusterStatus")]
pub struct DockerClusterSpec {
    /// ControlPlaneEndpoint represents the endpoint used to communicate with the control plane.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "controlPlaneEndpoint")]
    pub control_plane_endpoint: Option<DockerClusterControlPlaneEndpoint>,
    /// FailureDomains are usually not defined in the spec.
    /// The docker provider is special since failure domains don't mean anything in a local docker environment.
    /// Instead, the docker cluster controller will simply copy these into the Status and allow the Cluster API
    /// controllers to do what they will with the defined failure domains.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "failureDomains")]
    pub failure_domains: Option<BTreeMap<String, DockerClusterFailureDomains>>,
    /// LoadBalancer allows defining configurations for the cluster load balancer.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "loadBalancer")]
    pub load_balancer: Option<DockerClusterLoadBalancer>,
}

/// ControlPlaneEndpoint represents the endpoint used to communicate with the control plane.
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct DockerClusterControlPlaneEndpoint {
    /// Host is the hostname on which the API server is serving.
    pub host: String,
    /// Port is the port on which the API server is serving.
    /// Defaults to 6443 if not set.
    pub port: i64,
}

/// FailureDomains are usually not defined in the spec.
/// The docker provider is special since failure domains don't mean anything in a local docker environment.
/// Instead, the docker cluster controller will simply copy these into the Status and allow the Cluster API
/// controllers to do what they will with the defined failure domains.
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct DockerClusterFailureDomains {
    /// Attributes is a free form map of attributes an infrastructure provider might use or require.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attributes: Option<BTreeMap<String, String>>,
    /// ControlPlane determines if this failure domain is suitable for use by control plane machines.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "controlPlane")]
    pub control_plane: Option<bool>,
}

/// LoadBalancer allows defining configurations for the cluster load balancer.
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct DockerClusterLoadBalancer {
    /// CustomHAProxyConfigTemplateRef allows you to replace the default HAProxy config file.
    /// This field is a reference to a config map that contains the configuration template. The key of the config map should be equal to 'value'.
    /// The content of the config map will be processed and will replace the default HAProxy config file. Please use it with caution, as there are
    /// no checks to ensure the validity of the configuration. This template will support the following variables that will be passed by the controller:
    /// $IPv6 (bool) indicates if the cluster is IPv6, $FrontendControlPlanePort (string) indicates the frontend control plane port,
    /// $BackendControlPlanePort (string) indicates the backend control plane port, $BackendServers (map[string]string) indicates the backend server
    /// where the key is the server name and the value is the address. This map is dynamic and is updated every time a new control plane
    /// node is added or removed. The template will also support the JoinHostPort function to join the host and port of the backend server.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "customHAProxyConfigTemplateRef")]
    pub custom_ha_proxy_config_template_ref: Option<DockerClusterLoadBalancerCustomHaProxyConfigTemplateRef>,
    /// ImageRepository sets the container registry to pull the haproxy image from.
    /// if not set, "kindest" will be used instead.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "imageRepository")]
    pub image_repository: Option<String>,
    /// ImageTag allows to specify a tag for the haproxy image.
    /// if not set, "v20210715-a6da3463" will be used instead.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "imageTag")]
    pub image_tag: Option<String>,
}

/// CustomHAProxyConfigTemplateRef allows you to replace the default HAProxy config file.
/// This field is a reference to a config map that contains the configuration template. The key of the config map should be equal to 'value'.
/// The content of the config map will be processed and will replace the default HAProxy config file. Please use it with caution, as there are
/// no checks to ensure the validity of the configuration. This template will support the following variables that will be passed by the controller:
/// $IPv6 (bool) indicates if the cluster is IPv6, $FrontendControlPlanePort (string) indicates the frontend control plane port,
/// $BackendControlPlanePort (string) indicates the backend control plane port, $BackendServers (map[string]string) indicates the backend server
/// where the key is the server name and the value is the address. This map is dynamic and is updated every time a new control plane
/// node is added or removed. The template will also support the JoinHostPort function to join the host and port of the backend server.
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct DockerClusterLoadBalancerCustomHaProxyConfigTemplateRef {
    /// Name of the referent.
    /// More info: https://kubernetes.io/docs/concepts/overview/working-with-objects/names/#names
    /// TODO: Add other useful fields. apiVersion, kind, uid?
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// DockerClusterStatus defines the observed state of DockerCluster.
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct DockerClusterStatus {
    /// Conditions defines current service state of the DockerCluster.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<DockerClusterStatusConditions>>,
    /// FailureDomains don't mean much in CAPD since it's all local, but we can see how the rest of cluster API
    /// will use this if we populate it.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "failureDomains")]
    pub failure_domains: Option<BTreeMap<String, DockerClusterStatusFailureDomains>>,
    /// Ready denotes that the docker cluster (infrastructure) is ready.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ready: Option<bool>,
}

/// Condition defines an observation of a Cluster API resource operational state.
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct DockerClusterStatusConditions {
    /// Last time the condition transitioned from one status to another.
    /// This should be when the underlying condition changed. If that is not known, then using the time when
    /// the API field changed is acceptable.
    #[serde(rename = "lastTransitionTime")]
    pub last_transition_time: String,
    /// A human readable message indicating details about the transition.
    /// This field may be empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// The reason for the condition's last transition in CamelCase.
    /// The specific API may choose whether or not this field is considered a guaranteed API.
    /// This field may not be empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Severity provides an explicit classification of Reason code, so the users or machines can immediately
    /// understand the current situation and act accordingly.
    /// The Severity field MUST be set only when Status=False.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    /// Status of the condition, one of True, False, Unknown.
    pub status: String,
    /// Type of condition in CamelCase or in foo.example.com/CamelCase.
    /// Many .condition.type values are consistent across resources like Available, but because arbitrary conditions
    /// can be useful (see .node.status.conditions), the ability to deconflict is important.
    #[serde(rename = "type")]
    pub r#type: String,
}

/// FailureDomains don't mean much in CAPD since it's all local, but we can see how the rest of cluster API
/// will use this if we populate it.
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct DockerClusterStatusFailureDomains {
    /// Attributes is a free form map of attributes an infrastructure provider might use or require.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attributes: Option<BTreeMap<String, String>>,
    /// ControlPlane determines if this failure domain is suitable for use by control plane machines.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "controlPlane")]
    pub control_plane: Option<bool>,
}


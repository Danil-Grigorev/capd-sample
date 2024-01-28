use std::{
    collections::{hash_map::DefaultHasher, BTreeMap, HashMap},
    hash::{Hash, Hasher},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use docker_api::{
    models::NetworkingConfig,
    opts::{ContainerCreateOpts, ContainerCreateOptsBuilder, PublishPort},
};
use serde::Serialize;

use crate::{Error, Result};

// RunContainerInput holds the configuration settings for running a container.
#[derive(Debug, Clone, Default, Hash)]
pub struct RunContainerInput {
    /// Image is the name of the image to run.
    pub image: String,
    /// Name is the name to set for the container.
    pub name: String,
    /// Network is the name of the network to connect to.
    pub network: String,
    /// User is the user name to run as.
    pub user: Option<String>,
    /// Group is the user group to run as.
    pub group: Option<String>,
    /// Mount contains mount information for the container.
    pub mounts: Vec<Mount>,
    /// EnvironmentVars is a collection of name/values to pass as environment variables in the container.
    pub environment_vars: BTreeMap<String, String>,
    /// CommandArgs is the command and any additional arguments to execute in the container.
    pub command_args: Option<Vec<String>>,
    /// Entrypoint defines the entry point to use.
    pub entrypoint: Option<Vec<String>>,
    /// Labels to apply to the container.
    pub labels: BTreeMap<String, String>,
    /// PortMappings contains host<>container ports to map.
    /// IPFamily is the IP version to use.
    pub port_mappings: Vec<PortMapping>,
    // pub ip_family: ClusterIPFamily,
}

impl RunContainerInput {
    pub fn get_user(&self) -> String {
        match (self.user.clone(), self.group.clone()) {
            (None, None) => "".into(),
            (None, Some(_)) => "".into(),
            (Some(user), None) => user,
            (Some(user), Some(group)) => format!("{user}:{group}"),
        }
    }

    pub fn get_hash(self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

impl From<RunContainerInput> for ContainerCreateOpts {
    fn from(input: RunContainerInput) -> Self {
        let mut builder = ContainerCreateOptsBuilder::default()
            .user(input.get_user())
            .labels(input.labels)
            .network_mode(input.network)
            .hostname(input.name.clone())
            .name(input.name)
            .image(input.image)
            .env(input.environment_vars)
            .restart_policy("unless-stopped", 0)
            .privileged(true)
            .tty(true);

        // todo: volumes?
        if input.command_args.is_some() {
            builder = builder.command(input.command_args)
        }

        if input.entrypoint.is_some() {
            builder = builder.entrypoint(input.entrypoint)
        }

        // todo: validate network
        for port_mapping in input.port_mappings {
            builder = builder.expose(port_mapping.publish_port(), port_mapping.host_port);
        }

        if !input.mounts.is_empty() {
            let volumes: Vec<String> = input.mounts.iter().map(|v| v.to_string()).collect();
            builder = builder.volumes(volumes)
        }

        builder.build()
    }
}

// Mount contains mount details.
#[derive(Debug, Clone, Default, Serialize, Hash)]
pub struct Mount {
    // Source is the source host path to mount.
    pub source: String,
    // Target is the path to mount in the container.
    pub target: String,
    // ReadOnly specifies if the mount should be mounted read only.
    pub read_only: bool,
}

impl ToString for Mount {
    fn to_string(&self) -> String {
        match self {
            Mount {
                source,
                target,
                read_only: true,
            } => format!("{source}:{target}"),
            Mount {
                source,
                target,
                read_only: false,
            } => format!("{source}:{target}"),
        }
    }
}

// PortMapping contains port mapping information for the container.
#[derive(Debug, Clone, Default, Hash)]
pub struct PortMapping {
    // container_port is the port in the container to map to.
    pub container_port: u32,
    // host_port is the port to expose on the host.
    pub host_port: u32,
    // protocol is the protocol (tcp, udp, etc.) to use.
    pub protocol: String,
}

impl PortMapping {
    fn publish_port(&self) -> PublishPort {
        match self.protocol {
            _ => PublishPort::tcp(self.container_port),
        }
    }
}

// Define the ClusterIPFamily constants.
#[derive(Clone, Hash)]
pub enum ClusterIPFamily {
    IPv4IPFamily(Vec<Ipv4Addr>),
    IPv6IPFamily(Vec<Ipv6Addr>),
    DualStackIPFamily(Vec<IpAddr>),
}

impl Default for ClusterIPFamily {
    fn default() -> Self {
        Self::IPv4IPFamily(vec![])
    }
}

impl ClusterIPFamily {
    pub fn new(cidr_strings: Vec<String>) -> Result<ClusterIPFamily> {
        let ip_families: Result<Vec<IpAddr>> = cidr_strings
            .iter()
            .map(|c| c.parse::<IpAddr>().map_err(Error::IPFamilyUnknown))
            .collect();

        Ok(ClusterIPFamily::group(ip_families?))
    }

    fn group(ip_families: Vec<IpAddr>) -> ClusterIPFamily {
        match ip_families {
            ip_families if ip_families.iter().all(|ip| ip.is_ipv4()) => {
                ClusterIPFamily::IPv4IPFamily(
                    ip_families
                        .into_iter()
                        .filter_map(|ip| match ip {
                            IpAddr::V4(ip) => Some(ip),
                            _ => None,
                        })
                        .collect(),
                )
            }
            ip_families if ip_families.iter().all(|ip| ip.is_ipv6()) => {
                ClusterIPFamily::IPv6IPFamily(
                    ip_families
                        .into_iter()
                        .filter_map(|ip| match ip {
                            IpAddr::V6(ip) => Some(ip),
                            _ => None,
                        })
                        .collect(),
                )
            }
            ip_families => ClusterIPFamily::DualStackIPFamily(ip_families),
        }
    }
}

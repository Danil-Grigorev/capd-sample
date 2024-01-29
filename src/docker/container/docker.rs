use docker_api::{
    models::ContainerSummary,
    opts::{
        ContainerCreateOptsBuilder, ContainerFilter, ContainerListOpts, ContainerRemoveOptsBuilder,
        ExecCreateOptsBuilder, ExecStartOptsBuilder,
    },
    Container, Docker, Id,
};
use tracing::info;

use crate::{Error, Result, DEFAULT_DOCKER_SOCKET};

use super::interface::RunContainerInput;

#[derive(Clone)]
pub struct Runtime {
    client: Docker,
}

impl Runtime {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: Docker::new(format!("unix://{DEFAULT_DOCKER_SOCKET}"))
                .map_err(Error::ContainerError)?,
        })
    }

    pub fn get_container(&self, id: String) -> Container {
        self.client.containers().get(id)
    }

    pub async fn exec(&self, id: String, command: impl IntoIterator<Item = &str>) -> Result<()> {
        self.get_container(id)
            .exec(
                &ExecCreateOptsBuilder::default()
                    .command(command)
                    .attach_stderr(true)
                    .attach_stdout(true)
                    .privileged(true)
                    .build(),
                &ExecStartOptsBuilder::default().build(),
            )
            .await
            .map_err(Error::ContainerError)
            .map(|_| ())
    }

    pub async fn list_containers(
        &self,
        filters: Vec<ContainerFilter>,
    ) -> Result<Vec<ContainerSummary>> {
        self.client
            .containers()
            .list(&ContainerListOpts::builder().filter(filters).build())
            .await
            .map_err(Error::ContainerError)
    }

    pub async fn create_container(&self, run: RunContainerInput) -> Result<Container> {
        info!(
            "Creating container {} {:?}",
            run.image.clone(),
            run.labels.clone()
        );
        let container = self
            .client
            .containers()
            .create(&run.into())
            .await
            .map_err(Error::ContainerCreateError)?;

        container
            .start()
            .await
            .map_err(Error::ContainerCreateError)?;
        Ok(container)
    }

    pub async fn delete_container(&self, id: String) -> Result<String> {
        info!("Removing container");

        self.get_container(id)
            .remove(
                &ContainerRemoveOptsBuilder::default()
                    .force(true)
                    .volumes(true)
                    .build(),
            )
            .await
            .map_err(Error::ContainerRemoveError)
    }
}

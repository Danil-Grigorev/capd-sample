use std::{sync::Arc, time::Duration};

use futures::{StreamExt, TryStreamExt};
use kube::{
    runtime::{
        controller::Action,
        finalizer::{finalizer, Event},
        watcher::Config,
        Controller,
    },
    Api, Client,
};

use kube_core::{params::ListParams, ResourceExt};
use tracing::{error, info, instrument, warn};

use crate::{
    api::{cluster::Cluster, dockermachines::DockerMachine, machines::Machine},
    controllers::util::to_machine,
    Error, Result,
};

pub static MACHINE_CONTROLLER: &str = "cluster.x-k8s.io";

// Context for our reconciler
#[derive(Clone)]
pub struct Context {
    /// Kubernetes client
    pub client: Client,
}

/// Initialize the controller and shared state (given the crd is installed)
pub async fn run() {
    let client = Client::try_default()
        .await
        .expect("failed to create kube Client");
    let machines = Api::<DockerMachine>::all(client.clone());
    if let Err(e) = machines.list(&ListParams::default().limit(1)).await {
        error!("CRD is not queryable; {e:?}. Is the CRD installed?");
        std::process::exit(1);
    }

    Controller::new(machines, Config::default().any_semantic())
        .shutdown_on_signal()
        .watches(
            Api::<Machine>::all(client.clone()),
            Config::default(),
            to_machine,
        )
        // .watches_all(
        //     Api::<Cluster>::all(client.clone()),
        //     Default::default(),
        //     |_| true,
        //     |a, _| Some(ObjectRef::from_obj(a)),
        // )
        .run(reconcile, error_policy, Arc::new(Context { client }))
        .filter_map(|x| async move { std::result::Result::ok(x) })
        .for_each(|_| futures::future::ready(()))
        .await;
}

#[instrument(skip(ctx, machine), fields(trace_id))]
async fn reconcile(machine: Arc<DockerMachine>, ctx: Arc<Context>) -> Result<Action> {
    let ns = machine.namespace().unwrap();
    let machines: Api<DockerMachine> = Api::namespaced(ctx.client.clone(), &ns);

    info!(
        "Reconciling DockerMachine \"{}\" in {}",
        machine.name_any(),
        ns
    );
    finalizer(&machines, MACHINE_CONTROLLER, machine, |event| async {
        match event {
            Event::Apply(machine) => machine.reconcile(ctx).await, // machine.reconcile(ctx.clone()).await,
            Event::Cleanup(machine) => machine.cleanup(ctx).await, // machine.cleanup(ctx.clone()).await,
        }
    })
    .await
    .map_err(|e| Error::MachineError(Box::new(e)))
}

fn error_policy(_machine: Arc<DockerMachine>, error: &Error, _ctx: Arc<Context>) -> Action {
    warn!("reconcile failed: {:?}", error);
    Action::requeue(Duration::from_secs(5 * 60))
}

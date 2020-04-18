use futures::{StreamExt, TryStreamExt};
use serde::{
    Serialize,
    Deserialize
};

use kube::{
    api::{Api, WatchEvent},
    Client,
    runtime::Informer,
    config
};

use kube_derive::CustomResource;

#[derive(CustomResource, Serialize, Deserialize, Default, Clone, Debug)]
#[kube(
    group = "thames.id.au",
    kind = "Application",
    version = "v1beta1",
    namespaced
)]

pub struct ApplicationSpec {
    application: String,
    environment: String,
    version: String
}

fn handle(event: WatchEvent<Application>) -> anyhow::Result<()> {
    // This will receive events each time something 
    match event {
        WatchEvent::Added(application) => {
            println!("Added an application {:?} with version '{:?}'", application.metadata.name, application.spec.version)
        },
        WatchEvent::Deleted(application) => {
            println!("Deleted an application {:?}", application.metadata.name)
        }
        _ => {
            println!("another event")
        }
    }
    Ok(())
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load the kubeconfig file.
    let kubeconfig = config::Config::infer().await?;

    // Create a new client
    let client = Client::new(kubeconfig);

    // Set a namespace. We're just hard-coding for now.
    let namespace = "applications";

    // Describe the CRD we're working with.
    // This is basically the fields from our CRD definition.
    let applications: Api<Application> = Api::namespaced(client, namespace);
    let inform = Informer::new(applications);

    // Create our informer and start listening.
    loop {
        let mut events = inform.poll().await?.boxed();

        while let Some(event) = events.try_next().await? {
            handle(event)?;
        }
    }
}

use futures::{StreamExt, TryStreamExt, executor::block_on};
use serde::{
    Serialize,
    Deserialize
};
use serde_json::json;

use kube::{
    api::{Api, WatchEvent, PostParams},
    Client,
    runtime::Informer
};

use kube_derive::CustomResource;
use k8s_openapi::api::batch::v1::Job;
use structopt::StructOpt;

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

/// Search for a pattern in a file and display the lines that contain it.
#[derive(StructOpt)]
struct Cli {
    #[structopt(short = "n", long = "namespace", default_value = "applications")]
    namespace: String,
    #[structopt(short = "i", long = "image", env = "IMAGE")]
    image: String
}



fn ensure_application(client: Client, application: &Application, opts: &Cli) {

    let name = application.metadata.name.as_ref().expect("Application must have a name");
    let namespace = application.metadata.namespace.as_ref().expect(&format!("Application {} must be namespaced", name));
    let environment = &application.spec.environment;
    let version = &application.spec.version;
    let application_name = &application.spec.application;
    let config_version = std::env::var("CONFIG_VERSION").expect("CONFIG_VERSION environment variable must be set");
    let job_name = format!("{}-{}-{}",
                            name,
                            config_version.get(..8).or(Some(&config_version)).unwrap(),
                            &version.get(..8).or(Some(&version)).unwrap());
    println!(
        "Ensuring that application {:?} in environment {:?} has version {:?}",
        application_name,
        environment,
        version
    );
    let application_job = serde_json::from_value(json!({
        "apiVersion": "batch/v1",
        "kind": "Job",
        "metadata": {
            "name": job_name,
            "namespace": namespace
        },
        "spec": {
            "template": {
                "metadata": {
                    "name": job_name
                },
                "spec": {
                    "containers": [{
                        "name": "configurator",
                        "image": opts.image,
                        "command": [
                            "/bin/deploy",
                            application_name,
                            environment,
                            version
                        ]
                    }],
                    "restartPolicy": "Never",
                    "backoffLimit": 0
                }
            }
        }
    })).unwrap();
    let pp = PostParams::default();
    let jobs: Api<Job> = Api::namespaced(client, &namespace);
    let created = block_on(jobs.create(&pp, &application_job));
    match created {
        Ok(_) => (),
        Err(kube::Error::Api(ae)) => assert_eq!(ae.code, 409, "Couldn't create job"), // if you skipped delete, for instance
        Err(e) => panic!("Couldn't create job {:?}", e),                        // any other case is probably bad
    }
}

fn handle(client: Client, opts: &Cli, event: WatchEvent<Application>) -> anyhow::Result<()> {
    // This will receive events each time something 
    match event {
        WatchEvent::Added(application) | WatchEvent::Modified(application) => {
            ensure_application(client, &application, &opts);
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
    // Create a new client
    let client = Client::try_default().await?;

    let cli = &Cli::from_args();

    // Set a namespace. We're just hard-coding for now.
    let namespace = &cli.namespace;

    // Describe the CRD we're working with.
    // This is basically the fields from our CRD definition.
    let applications: Api<Application> = Api::namespaced(client.clone(), &namespace);
    let inform = Informer::new(applications);

    // Create our informer and start listening.
    loop {
        let mut events = inform.poll().await?.boxed();

        while let Some(event) = events.try_next().await? {
            handle(client.clone(), cli, event)?;
        }
    }
}

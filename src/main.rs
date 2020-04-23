use futures::{StreamExt, TryStreamExt, executor::block_on};
use handlebars::Handlebars;
use std::path::PathBuf;
use std::fs;
use serde::{
    Serialize,
    Deserialize
};
use serde_yaml;

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
    group = "application-operator.github.io",
    kind = "Application",
    version = "v1alpha1",
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
    image: String,
    #[structopt(short = "c", long = "command", default_value = "/bin/deploy")]
    command: String,
    #[structopt(short = "s", long = "service-account", default_value = "default")]
    service_account: String,
    #[structopt(short = "t", long = "template", parse(from_os_str))]
    template: PathBuf,
}

#[derive(Serialize)]
struct TemplateVars {
    application: String,
    environment: String,
    version: String,
    command: String,
    job_name: String,
    namespace: String,
    service_account: String,
    image: String
}

fn version_to_rfc1123(version: String, length: usize) -> String {
    let version = version.replace(".", "-").replace("_", "-");
    return version.get(..length).or(Some(&version)).unwrap().trim_end_matches("-").to_string();
}

fn ensure_application(client: Client, application: &Application, opts: &Cli) {

    let name = application.metadata.name.as_ref().expect("Application must have a name");
    let version = application.spec.version.clone();
    let config_version = std::env::var("CONFIG_VERSION").expect("CONFIG_VERSION environment variable must be set");
    let namespace = application.metadata.namespace.as_ref().expect(&format!("Application {} must be namespaced", name));

    let template_vars = TemplateVars {
        application: application.spec.application.clone(),
        environment: application.spec.environment.clone(),
        version: version.clone(),
        command: opts.command.clone(),
        namespace: namespace.to_string(),
        job_name: format!("{}-{}-{}", name, version_to_rfc1123(config_version, 20), version_to_rfc1123(version, 20)),
        service_account: opts.service_account.clone(),
        image: opts.image.clone()
    };

    println!(
        "Ensuring that application {:?} in environment {:?} has version {:?}",
        template_vars.application,
        template_vars.environment,
        template_vars.version
    );
    let reg = Handlebars::new();
    let contents = fs::read_to_string(&opts.template)
        .expect("Something went wrong reading the file");
    let application_job : Job = serde_yaml::from_str(&reg.render_template(&contents, &template_vars).unwrap()).unwrap();
    let pp = PostParams::default();
    let jobs: Api<Job> = Api::namespaced(client, &namespace);
    let created = block_on(jobs.create(&pp, &application_job));
    match created {
        Ok(_) => (),
        Err(kube::Error::Api(ae)) => assert_eq!(ae.code, 409, "Couldn't create job: {:?}", ae.message),
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_to_rfc1123_test() {
        assert_eq!("hello-world".to_string(), version_to_rfc1123("hello-world".to_string(), 20));
        assert_eq!("hello-worl".to_string(), version_to_rfc1123("hello-world".to_string(), 10));
        assert_eq!("hello".to_string(), version_to_rfc1123("hello-world".to_string(), 6));
        assert_eq!("hello-1234".to_string(), version_to_rfc1123("hello.1234".to_string(), 10));
        assert_eq!("hello".to_string(), version_to_rfc1123("hello-----".to_string(), 10));
    }
}

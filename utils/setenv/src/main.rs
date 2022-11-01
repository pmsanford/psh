use std::{env, path::PathBuf, process::exit};

use anyhow::Result;
use protos::{env_client::EnvClient, EnvVar};
use tokio::net::UnixStream;
use tonic::{
    transport::{Endpoint, Uri},
    Request,
};
use tower::service_fn;

#[tokio::main]
async fn main() -> Result<()> {
    let args = env::args();
    if args.len() < 3 {
        eprintln!("Requires at least two args: A var name and a value");
    }
    let mut args = args.skip(1);
    let key = args.next().unwrap();
    let value = args.into_iter().collect::<Vec<_>>();
    let value = value.join(" ");
    if !matches!(
        std::env::var("PSH_SERVICE_SOCK")
            .map(|path| PathBuf::from(path).exists())
            .ok(),
        Some(true)
    ) {
        eprintln!("Couldn't find a service socket. Are you running from within psh?");
        exit(1);
    }
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(|_: Uri| {
            let sock_path = std::env::var("PSH_SERVICE_SOCK").unwrap();
            UnixStream::connect(sock_path)
        }))
        .await?;

    let mut client = EnvClient::new(channel);

    let request = Request::new(EnvVar { key, value });

    client.set_env(request).await?;

    Ok(())
}

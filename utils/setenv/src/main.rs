use std::{env, path::PathBuf, process::exit};

use anyhow::Result;
use protos::{create_channel, env_client::EnvClient, sock_path_from_env, EnvVar};
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
    if !sock_path_from_env().map(|p| p.exists()).unwrap_or(false) {
        eprintln!("Couldn't find a service socket. Are you running from within psh?");
        exit(1);
    }
    let channel = create_channel(sock_path_from_env().unwrap()).await?;

    let mut client = EnvClient::new(channel);

    let request = Request::new(EnvVar { key, value });

    client.set_env(request).await?;

    Ok(())
}

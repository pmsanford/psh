tonic::include_proto!("env");
use std::{fs::create_dir_all, path::PathBuf};

use anyhow::{anyhow, bail, Result};
use tokio::net::UnixStream;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;

pub fn create_sock_path() -> Result<PathBuf> {
    let path = sock_path_from_pid(std::process::id());
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("parent doesn't exist"))?;
    if !parent.exists() {
        create_dir_all(parent)?;
    }

    Ok(path)
}

pub fn sock_path_from_pid(pid: u32) -> PathBuf {
    PathBuf::from(format!("/tmp/psh/{}/service.sock", pid))
}

pub fn sock_path_from_env() -> Result<PathBuf> {
    Ok(std::env::var("PSH_SERVICE_SOCK").map(PathBuf::from)?)
}

pub async fn create_channel(sock_path: PathBuf) -> Result<Channel> {
    if !sock_path.exists() {
        bail!("Socket does not exist");
    }
    Ok(Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(move |_: Uri| {
            let sock_path = sock_path.clone();
            UnixStream::connect(sock_path)
        }))
        .await?)
}

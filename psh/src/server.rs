use std::path::PathBuf;

use anyhow::Result;
use protos::{
    env_server::{Env, EnvServer},
    EnvVar,
};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{transport::Server, Request, Response, Status};

struct EnvListener;

#[tonic::async_trait]
impl Env for EnvListener {
    async fn set_env(&self, req: Request<EnvVar>) -> Result<Response<()>, Status> {
        let req = req.into_inner();
        std::env::set_var(req.key, req.value);

        Ok(Response::new(()))
    }
}

pub async fn start_env_service() -> Result<()> {
    let mut sock_path = PathBuf::from("/tmp/psh/");
    sock_path.push(std::process::id().to_string());
    if !sock_path.exists() {
        std::fs::create_dir_all(&sock_path)?;
    }
    sock_path.push("service.sock");
    std::env::set_var("PSH_SERVICE_SOCK", sock_path.to_string_lossy().to_string());
    let env_listener = EnvListener;

    let uds = UnixListener::bind(sock_path)?;
    let uds_stream = UnixListenerStream::new(uds);

    Server::builder()
        .add_service(EnvServer::new(env_listener))
        .serve_with_incoming(uds_stream)
        .await?;

    Ok(())
}

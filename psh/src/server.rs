use anyhow::Result;
use protos::{
    create_sock_path,
    env_server::{Env, EnvServer},
    EnvVar, GetEnvResponse,
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

    async fn get_env(&self, _req: Request<()>) -> Result<Response<GetEnvResponse>, Status> {
        let mut vars = vec![];

        for (key, value) in std::env::vars() {
            vars.push(EnvVar { key, value });
        }

        Ok(Response::new(GetEnvResponse { vars }))
    }
}

pub async fn start_env_service() -> Result<()> {
    let sock_path = create_sock_path()?;
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

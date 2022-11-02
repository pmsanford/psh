use std::sync::{Arc, Mutex};

use anyhow::Result;
use protos::{
    create_sock_path,
    env_server::{Env, EnvServer},
    status_server::{Status as StatusTrait, StatusServer},
    EnvVar, GetEnvResponse, GetStatusResponse,
};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{transport::Server, Request, Response, Status};

use crate::state::State;

struct StatusListener {
    state: Arc<Mutex<State>>,
}

#[tonic::async_trait]
impl StatusTrait for StatusListener {
    async fn get_status(&self, _req: Request<()>) -> Result<Response<GetStatusResponse>, Status> {
        let state = self.state.lock().unwrap();
        let working_dir = std::env::current_dir()
            .map(|wd| wd.to_string_lossy().to_string())
            .unwrap_or_else(|_| String::from("<none>"))
            .to_string();
        Ok(Response::new(GetStatusResponse {
            current_command: state
                .current_command
                .clone()
                .unwrap_or_else(|| String::from("<none>")),
            working_dir,
        }))
    }
}

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

pub async fn start_services(state: Arc<Mutex<State>>) -> Result<()> {
    let sock_path = create_sock_path()?;
    std::env::set_var("PSH_SERVICE_SOCK", sock_path.to_string_lossy().to_string());
    let env_listener = EnvListener;
    let status_listener = StatusListener { state };

    let uds = UnixListener::bind(sock_path)?;
    let uds_stream = UnixListenerStream::new(uds);

    Server::builder()
        .add_service(EnvServer::new(env_listener))
        .add_service(StatusServer::new(status_listener))
        .serve_with_incoming(uds_stream)
        .await?;

    Ok(())
}

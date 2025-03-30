use flo_controller::{serve_grpc, serve_socket, ControllerState};
use flo_otel::builder::{Environment, OtelBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  #[cfg(debug_assertions)]
  {
    dotenv::dotenv()?;
  }

  let _guard = OtelBuilder::new()
      .set_environment(Environment::Dev)
      .set_token("".to_string())
      .set_project_name(env!("CARGO_PKG_NAME").to_string())
      .set_version(env!("CARGO_PKG_VERSION").to_string())
      .build()?;

  tracing::info!("Starting flo-controller-service");
  let state = ControllerState::init().await?.into_ref();

  #[cfg(unix)]
  {
    use tokio::signal::unix::{signal, SignalKind};
    let mut stream = signal(SignalKind::hangup())?;
    tokio::spawn({
      let state = state.clone();
      async move {
        loop {
          stream.recv().await;
          tracing::info!("reloading");
          if let Err(err) = state.reload().await {
            tracing::error!("reload error: {}", err);
          }
        }
      }
    });
  }

  tracing::info!("Initiating gRPC and socket serving");
  tokio::try_join!(serve_grpc(state.clone()), serve_socket(state.clone()))?;

  Ok(())
}

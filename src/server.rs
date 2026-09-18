use anyhow::Result;
use axum::{
    extract::Query,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;
use tracing::{error, info};

use crate::bot::{send_bot_command, BotCommand};
use crate::discovery::find_target_bot;

#[derive(Deserialize, Debug, Default)]
pub struct DeviceParams {
    pub device: Option<String>,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "switchbot-bridge",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn execute_command(cmd: BotCommand, params: DeviceParams) -> (StatusCode, Json<StatusResponse>) {
    let filter = params.device.as_deref();
    info!("HTTP request: execute {:?} on device filter: {:?}", cmd.name(), filter);

    match find_target_bot(filter, Duration::from_secs(6)).await {
        Ok(peripheral) => {
            let id = peripheral.id().to_string();
            match send_bot_command(&peripheral, cmd).await {
                Ok(_) => (
                    StatusCode::OK,
                    Json(StatusResponse {
                        success: true,
                        message: format!("Successfully sent {} to SwitchBot Bot", cmd.name()),
                        device: Some(id),
                    }),
                ),
                Err(e) => {
                    error!("Failed sending command to Bot: {:?}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(StatusResponse {
                            success: false,
                            message: format!("Command failed: {}", e),
                            device: Some(id),
                        }),
                    )
                }
            }
        }
        Err(e) => {
            error!("Could not find target SwitchBot: {:?}", e);
            (
                StatusCode::NOT_FOUND,
                Json(StatusResponse {
                    success: false,
                    message: format!("Device discovery failed: {}", e),
                    device: None,
                }),
            )
        }
    }
}

async fn handle_press(Query(params): Query<DeviceParams>) -> impl IntoResponse {
    execute_command(BotCommand::Press, params).await
}

async fn handle_on(Query(params): Query<DeviceParams>) -> impl IntoResponse {
    execute_command(BotCommand::On, params).await
}

async fn handle_off(Query(params): Query<DeviceParams>) -> impl IntoResponse {
    execute_command(BotCommand::Off, params).await
}

pub async fn run_server(host: &str, port: u16) -> Result<()> {
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/press", post(handle_press))
        .route("/on", post(handle_on))
        .route("/off", post(handle_off))
        // Convenience aliases for sauna
        .route("/sauna/press", post(handle_press))
        .route("/sauna/on", post(handle_on))
        .route("/sauna/off", post(handle_off));

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    info!("🚀 SwitchBot Bridge HTTP server running at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

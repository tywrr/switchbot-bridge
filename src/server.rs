use anyhow::Result;
use axum::{
    extract::Query,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use btleplug::api::Peripheral as _;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

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

/// A Bot that just finished a BLE session (e.g. the "off" press 10 s earlier)
/// can take several seconds to resume advertising, and CoreBluetooth does not
/// always re-list it within one scan. Retry discovery and the command a few
/// times before reporting failure; Home Assistant waits for the power-draw
/// change, not for this response, so a slower success is still a success.
const COMMAND_ATTEMPTS: u32 = 3;
const SCAN_TIMEOUT: Duration = Duration::from_secs(6);
const RETRY_PAUSE: Duration = Duration::from_millis(1500);

async fn execute_command(cmd: BotCommand, params: DeviceParams) -> (StatusCode, Json<StatusResponse>) {
    let filter = params.device.as_deref();
    info!("HTTP request: execute {:?} on device filter: {:?}", cmd.name(), filter);

    let mut last: Option<(StatusCode, String, Option<String>)> = None;
    for attempt in 1..=COMMAND_ATTEMPTS {
        if let Some((_, msg, _)) = &last {
            warn!("Attempt {}/{} after: {}", attempt, COMMAND_ATTEMPTS, msg);
            sleep(RETRY_PAUSE).await;
        }
        let peripheral = match find_target_bot(filter, SCAN_TIMEOUT).await {
            Ok(p) => p,
            Err(e) => {
                last = Some((StatusCode::NOT_FOUND, format!("Device discovery failed: {}", e), None));
                continue;
            }
        };
        let id = peripheral.id().to_string();
        match send_bot_command(&peripheral, cmd).await {
            Ok(_) => {
                return (
                    StatusCode::OK,
                    Json(StatusResponse {
                        success: true,
                        message: format!("Successfully sent {} to SwitchBot Bot (attempt {})", cmd.name(), attempt),
                        device: Some(id),
                    }),
                )
            }
            Err(e) => {
                let _ = peripheral.disconnect().await;
                last = Some((StatusCode::INTERNAL_SERVER_ERROR, format!("Command failed: {}", e), Some(id)));
                continue;
            }
        }
    }
    let (status, message, device) = last.expect("at least one attempt");
    error!("Giving up after {} attempts: {}", COMMAND_ATTEMPTS, message);
    (status, Json(StatusResponse { success: false, message, device }))
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

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{HeaderValue, Response, StatusCode, Uri, header},
    response::IntoResponse,
    routing::{get, post},
};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use tokio::signal;
use tower_http::cors::CorsLayer;
use tracing::{info, warn};

use crate::vm::{VmApi, handlers};

// Embed the UI assets directly into the binary
#[derive(RustEmbed)]
#[folder = "ui/"]
struct UiAssets;

#[derive(Clone)]
pub struct AppState {
    pub(crate) vm_api: Arc<dyn VmApi>,
}

impl AppState {
    pub fn new(vm_api: Arc<dyn VmApi>) -> Self {
        Self { vm_api }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VmStatusDto {
    pub name: String,
    pub state: String,
    pub ipv4: Option<Vec<String>>,
    pub release: Option<String>,
    pub memory_total: Option<u64>,
    pub memory_used: Option<u64>,
    pub disk_total: Option<u64>,
    pub disk_used: Option<u64>,
}

// REST API handlers
async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "ok"})))
}

async fn list_vms(State(state): State<AppState>) -> impl IntoResponse {
    match state.vm_api.list().await {
        Ok(vms) => {
            let dtos: Vec<VmStatusDto> = vms
                .into_iter()
                .map(|vm| VmStatusDto {
                    name: vm.name,
                    state: vm.state,
                    ipv4: vm.ipv4,
                    release: vm.release,
                    memory_total: None,
                    memory_used: None,
                    disk_total: None,
                    disk_used: None,
                })
                .collect();
            (StatusCode::OK, Json(dtos)).into_response()
        }
        Err(e) => {
            warn!("failed to list VMs: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("{}", e)})),
            )
                .into_response()
        }
    }
}

async fn get_vm_info(
    State(state): State<AppState>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> impl IntoResponse {
    match state.vm_api.info(&name).await {
        Ok(info) => {
            let dto = VmStatusDto {
                name: info.name,
                state: info.state,
                ipv4: info.ipv4,
                release: info.release,
                memory_total: info.memory_total,
                memory_used: info.memory_used,
                disk_total: info.disk_total,
                disk_used: info.disk_used,
            };
            (StatusCode::OK, Json(dto)).into_response()
        }
        Err(e) => {
            warn!("failed to get VM info for {}: {}", name, e);
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": format!("{}", e)})),
            )
                .into_response()
        }
    }
}

#[derive(Debug, Deserialize)]
struct LaunchVmRequest {
    name: String,
}

async fn launch_vm(
    State(state): State<AppState>,
    Json(payload): Json<LaunchVmRequest>,
) -> impl IntoResponse {
    let result = handlers::launch_vm(state.vm_api.as_ref(), &payload.name).await;
    if result.success {
        (
            StatusCode::CREATED,
            Json(serde_json::json!({"success": true, "message": result.message})),
        )
            .into_response()
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"success": false, "error": result.message})),
        )
            .into_response()
    }
}

async fn start_vm(
    State(state): State<AppState>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> impl IntoResponse {
    let result = handlers::start_vm(state.vm_api.as_ref(), &name).await;
    if result.success {
        (
            StatusCode::OK,
            Json(serde_json::json!({"success": true, "message": result.message})),
        )
            .into_response()
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"success": false, "error": result.message})),
        )
            .into_response()
    }
}

async fn stop_vm(
    State(state): State<AppState>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> impl IntoResponse {
    let result = handlers::stop_vm(state.vm_api.as_ref(), &name).await;
    if result.success {
        (
            StatusCode::OK,
            Json(serde_json::json!({"success": true, "message": result.message})),
        )
            .into_response()
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"success": false, "error": result.message})),
        )
            .into_response()
    }
}

async fn restart_vm(
    State(state): State<AppState>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> impl IntoResponse {
    let result = handlers::restart_vm(state.vm_api.as_ref(), &name).await;
    if result.success {
        (
            StatusCode::OK,
            Json(serde_json::json!({"success": true, "message": result.message})),
        )
            .into_response()
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"success": false, "error": result.message})),
        )
            .into_response()
    }
}

async fn delete_vm(
    State(state): State<AppState>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> impl IntoResponse {
    let result = handlers::delete_vm(state.vm_api.as_ref(), &name).await;
    if result.success {
        (
            StatusCode::OK,
            Json(serde_json::json!({"success": true, "message": result.message})),
        )
            .into_response()
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"success": false, "error": result.message})),
        )
            .into_response()
    }
}

pub fn create_api_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/vms", get(list_vms).post(launch_vm))
        .route("/vms/{name}", get(get_vm_info).delete(delete_vm))
        .route("/vms/{name}/start", post(start_vm))
        .route("/vms/{name}/stop", post(stop_vm))
        .route("/vms/{name}/restart", post(restart_vm))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

pub fn create_ui_router() -> Router {
    Router::new().fallback(serve_embedded_file)
}

async fn serve_embedded_file(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();

    // Default to index.html if path is empty or ends with /
    if path.is_empty() || path.ends_with('/') {
        path = "index.html".to_string();
    }

    match UiAssets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            let body = Body::from(content.data.into_owned());

            Response::builder()
                .status(StatusCode::OK)
                .header(
                    header::CONTENT_TYPE,
                    HeaderValue::from_str(mime.as_ref()).unwrap(),
                )
                .body(body)
                .unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("404 Not Found"))
            .unwrap(),
    }
}

pub async fn run_server(
    vm_api: Arc<dyn VmApi>,
    host: &str,
    ui_port: u16,
    api_port: u16,
) -> Result<()> {
    let state = AppState::new(vm_api);

    // Parse host address
    let host_addr: std::net::IpAddr = host
        .parse()
        .context(format!("invalid host address: {}", host))?;

    // API server
    let api_router = create_api_router(state.clone());
    let api_addr = SocketAddr::from((host_addr, api_port));

    // UI server (using embedded assets)
    let ui_router = create_ui_router();
    let ui_addr = SocketAddr::from((host_addr, ui_port));

    info!(
        "🏡 Starting SafePaw village UI on http://{}:{}",
        host, ui_port
    );
    info!(
        "📡 Starting REST API server on http://{}:{}",
        host, api_port
    );
    info!("🌐 Visit the UI to access the SafePaw village");
    info!("🔌 API health check: http://{}:{}/health", host, api_port);

    // Spawn both servers concurrently
    let api_server = async {
        let listener = tokio::net::TcpListener::bind(api_addr)
            .await
            .context(format!(
                "failed to bind API server to {}:{}",
                host, api_port
            ))?;
        axum::serve(listener, api_router)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .context("API server failed")
    };

    let ui_server = async {
        let listener = tokio::net::TcpListener::bind(ui_addr)
            .await
            .context(format!("failed to bind UI server to {}:{}", host, ui_port))?;
        axum::serve(listener, ui_router)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .context("UI server failed")
    };

    tokio::try_join!(api_server, ui_server)?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down gracefully");
        }
        _ = terminate => {
            info!("Received terminate signal, shutting down gracefully");
        }
    }
}

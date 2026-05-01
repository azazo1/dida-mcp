use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use axum::{
    Json as AxumJson, Router,
    extract::State,
    http::{HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::Utc;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use serde::Serialize;
use tokio::net::TcpListener;
use tracing::{info, warn};

use crate::{
    config::{
        AppConfig, ServerConfig, load_config, normalized_token, resolve_config_path,
        validate_config,
    },
    proxy::DidaProxy,
};

#[derive(Serialize)]
struct HealthzResponse<'a> {
    status: &'a str,
    server_time: String,
}

pub async fn run() -> Result<()> {
    init_tracing();

    let config_path = resolve_config_path();
    let config = Arc::new(load_config(&config_path)?);
    validate_config(&config)?;

    run_with_config(config).await
}

pub async fn run_with_config(config: Arc<AppConfig>) -> Result<()> {
    let proxy = DidaProxy::new(config.clone());
    let service: StreamableHttpService<DidaProxy, LocalSessionManager> = StreamableHttpService::new(
        move || Ok(proxy.clone()),
        LocalSessionManager::default().into(),
        build_mcp_config(&config.server),
    );

    let mcp_router = Router::new().nest_service(config.server.base_path.as_str(), service);
    let app = if let Some(token) = normalized_token(config.server.inbound_bearer_token.as_deref()) {
        Router::new()
            .route("/healthz", get(healthz))
            .merge(mcp_router.layer(middleware::from_fn_with_state(
                Arc::new(token.to_owned()),
                auth_middleware,
            )))
    } else {
        Router::new()
            .route("/healthz", get(healthz))
            .merge(mcp_router)
    };

    let addr: SocketAddr = config
        .server
        .listen
        .parse()
        .with_context(|| format!("invalid server.listen: {}", config.server.listen))?;

    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind {}", config.server.listen))?;

    info!(
        listen = %config.server.listen,
        base_path = %config.server.base_path,
        remote_url = %config.remote.url,
        "starting dida MCP proxy",
    );

    axum::serve(listener, app)
        .await
        .context("axum server exited unexpectedly")
}

fn build_mcp_config(server: &ServerConfig) -> StreamableHttpServerConfig {
    let mut mcp_config = StreamableHttpServerConfig::default()
        .with_stateful_mode(server.stateful_mode)
        .with_sse_keep_alive(Some(Duration::from_secs(server.sse_keep_alive_secs)));

    if server.disable_host_validation {
        mcp_config = mcp_config.disable_allowed_hosts();
    }

    mcp_config
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,rmcp=warn,hyper=warn".into());

    tracing_subscriber::fmt().with_env_filter(filter).init();
}

async fn healthz() -> impl IntoResponse {
    AxumJson(HealthzResponse {
        status: "ok",
        server_time: Utc::now().to_rfc3339(),
    })
}

async fn auth_middleware(
    State(expected_token): State<Arc<String>>,
    headers: HeaderMap,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    match extract_bearer_token(&headers) {
        Some(actual) if actual == expected_token.as_str() => Ok(next.run(request).await),
        _ => {
            warn!("rejected request due to missing or invalid inbound bearer token");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    let header = headers.get("Authorization")?.to_str().ok()?;
    header.strip_prefix("Bearer ")
}

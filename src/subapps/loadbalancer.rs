use crate::config::loadbalancer_config::{Features, LoadBalancerConfig, Protocol};
use crate::db_ops::lb_db::{insert_apis, update_error_code, update_hit};
use crate::validator::validate::{read_json_from_file, validate_person_json};

use axum::{
    body::{to_bytes, Body},
    extract::{Path, State},
    http::{Request, Response, StatusCode},
    routing::get,
    Router,
};
use log::{error, info, warn};
use reqwest::Client;

use crate::common::background::{api_health_check, health_check, load_balancer_connections};
use serde::Deserialize;
use serde_json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::fs;
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    sync::Arc,
};
use tokio::net::TcpListener;

#[derive(Clone)]
struct LoadBalancerState {
    ip: String,
    servers: Arc<Vec<String>>,
    protocol: Arc<Protocol>,
    features: Arc<Vec<Features>>,
    index: Arc<AtomicUsize>,
    client: Client,
    db: PgPool,
}

#[derive(Debug, Deserialize)]
pub struct ApiConfig {
    pub apis: Vec<Api>,
    pub check_interval_ms: u64,
    pub timeout_ms: u64,
    pub failure_threshold: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Api {
    pub url: String,
    pub method: String,
    #[serde(default)]
    pub body: Option<HashMap<String, String>>,
}

impl LoadBalancerState {
    fn new(db: PgPool) -> Self {
        let cfg: LoadBalancerConfig =
            confy::load("load-balancer-config", None).expect("failed to load config");
        let ip = cfg.ip;
        let protocol = Arc::new(cfg.protocol);
        let features = Arc::new(cfg.features.clone());
        let index = Arc::new(AtomicUsize::new(0));
        let servers = Arc::new(cfg.nodes);

        LoadBalancerState {
            ip,
            servers,
            protocol,
            features,
            index,
            client: Client::new(),
            db,
        }
    }

    async fn forward_request(&self, req: Request<Body>) -> Result<Response<String>, StatusCode> {
        let (parts, body) = req.into_parts();

        let original_path = parts.uri.path();
        let server_index = self.index.fetch_add(1, Ordering::SeqCst) % self.servers.len();
        let server_url = &self.servers[server_index];

        let uri = format!("{}{}", server_url, parts.uri);
        let method = parts.method.clone();
        let body = to_bytes(body, usize::MAX)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        let response = self
            .client
            .request(method, &uri)
            .body(body)
            .send()
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;

        let status = response.status();
        if !status.is_success() {
            match update_error_code(&original_path, &status, &self.db).await {
                Ok(()) => {}
                Err(e) => {
                    warn!("error code failed to update, moitering might not work as expected!");
                    error!("{}", e);
                }
            }
        }
        info!("status code for url {}: {}", uri, status);
        let body = response.text().await.unwrap_or_else(|_| "".to_string());

        Ok(Response::builder().status(status).body(body).unwrap())
    }
}

pub async fn balance_load(db: PgPool) {
    //todo make switch to protocol
    let load_balancer_state = LoadBalancerState::new(db);
    let address = load_balancer_state.clone().ip + ":3000";
    let servers = load_balancer_state.clone().servers;
    tokio::spawn(load_balancer_connections());
    if load_balancer_state
        .features
        .contains(&Features::HealthCheck)
    {
        tokio::spawn(health_check(servers));
    }

    if load_balancer_state
        .features
        .contains(&Features::ApiHealthCheck)
    {
        let file_path = "./src/subapps/api.json";

        match read_json_from_file(file_path) {
            Ok(json_str) => match validate_person_json(&json_str) {
                Ok(config) => info!("✅ Valid JSON: {:#?}", config),
                Err(e) => error!("❌ Invalid JSON structure: {}", e),
            },
            Err(e) => error!("❌ Failed to read file: {}", e),
        }

        let data = fs::read_to_string(file_path).expect("unable to read file");

        let api_config: ApiConfig = serde_json::from_str(&data).expect("Unable to parse JSON");
        let apis = api_config.apis.clone();

        for api in apis.iter() {
            info!("{:#?}", api);
        }

        match insert_apis(&apis, &load_balancer_state.db).await {
            Ok(()) => {
                tokio::spawn(api_health_check(api_config));
            }
            Err(e) => {
                warn!("apis not inserted, monitering might not work as expected");
                error!("{}", e);
                tokio::spawn(api_health_check(api_config));
            }
        };
    }

    let app = Router::new()
        .route("/{*wildcard}", get(handle_request).post(handle_request))
        .with_state(load_balancer_state);

    let listener = TcpListener::bind(address)
        .await
        .expect("failed to listen...");
    info!("loadbalancer is listening...");
    axum::serve(listener, app).await.unwrap();
}

async fn handle_request(
    Path(_path): Path<String>,
    lb: State<LoadBalancerState>,
    req: Request<Body>,
) -> Result<Response<String>, StatusCode> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = uri.path();
    let query = uri.query().unwrap_or("");

    match update_hit(path, &lb.db).await {
        Ok(()) => {}
        Err(e) => {
            warn!("failed to update hit, metrics might return incorrect results");
            error!("{}", e);
        }
    };

    info!("Incoming request: {} {}?{}", method, path, query);

    lb.forward_request(req).await
}

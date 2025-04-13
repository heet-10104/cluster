use crate::config::loadbalancer_config::{Features, LoadBalancerConfig, Protocol};
use crate::subapps::node::Metrics;
use axum::http::response;
use axum::{
    body::{to_bytes, Body},
    extract::{Path, State},
    http::{Request, Response, StatusCode},
    routing::get,
    Router,
};
use reqwest::Client;
use std::thread::sleep;
use std::time::Duration;
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
}

impl LoadBalancerState {
    fn new() -> Self {
        let cfg: LoadBalancerConfig =
            confy::load("load-balancer-config", None).expect("Failed to load config");
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
        }
    }

    async fn forward_request(&self, req: Request<Body>) -> Result<Response<String>, StatusCode> {
        let server_index = self.index.fetch_add(1, Ordering::SeqCst) % self.servers.len();
        let server_url = &self.servers[server_index];

        let uri = format!("{}{}", server_url, req.uri());
        let method = req.method().clone();
        let body = to_bytes(req.into_body(), usize::MAX)
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
        let body = response.text().await.unwrap_or_else(|_| "".to_string());

        Ok(Response::builder().status(status).body(body).unwrap())
    }
}

async fn health_check(servers: Vec<String>) {
    loop {
        for ip in &servers {
            let url = "http://".to_owned() + &ip + ":3000" + "/metrics";
            let client = Client::new();

            let response = client.get(url).send().await.expect("metrtics from server");
            let metrics: Metrics = response.json().await.expect("failed to parse JSON");
            println!("{:?}", metrics);
        }

        sleep(Duration::from_secs(60));
    }
}

pub async fn balance_load() {
    //todo make switch to protocol
    let load_balancer_state = LoadBalancerState::new();
    let address = load_balancer_state.clone().ip + ":3000";
    let servers = load_balancer_state.clone().servers;
    match Arc::try_unwrap(servers) {
        Ok(servers) => {
            tokio::spawn(async move {
                health_check(servers).await;
            });
        }
        Err(_) => {
            println!("Arc has other owners, can't unwrap.");
        }
    }

    let app = Router::new()
        .route("/{*wildcard}", get(handle_request).post(handle_request))
        .with_state(load_balancer_state);
    let listener = TcpListener::bind(address).await.unwrap();
    println!("loadbalancer is listening...");
    axum::serve(listener, app).await.unwrap();
}

async fn handle_request(
    Path(path): Path<String>,
    lb: State<LoadBalancerState>,
    req: Request<Body>,
) -> Result<Response<String>, StatusCode> {
    lb.forward_request(req).await
}

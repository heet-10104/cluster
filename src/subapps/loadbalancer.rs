use crate::config::loadbalancer_config::{Features, LoadBalancerConfig, Protocol};
use crate::subapps::node::Metrics;
use axum::{
    body::{to_bytes, Body},
    extract::{Path, State},
    http::{Request, Response, StatusCode},
    routing::get,
    Router,
};
use core::time;
use reqwest::redirect::Action;
use reqwest::Client;
use serde::de::Expected;
use serde::Deserialize;
use std::collections::HashMap;
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    sync::Arc,
    thread::sleep,
    time::Duration,
};
use tokio::net::TcpListener;

use serde_json;
use std::fs;

#[derive(Clone)]
struct LoadBalancerState {
    ip: String,
    servers: Arc<Vec<String>>,
    protocol: Arc<Protocol>,
    features: Arc<Vec<Features>>,
    index: Arc<AtomicUsize>,
    client: Client,
}

#[derive(Debug, Deserialize)]
struct ApiConfig {
    apis: Vec<Api>,
    check_interval_ms: u64,
    timeout_ms: u64,
    expected_body_contains: String,
    failure_threshold: u32,
}

#[derive(Debug, Deserialize, Clone)]
struct Api {
    url: String,
    method: String,
    #[serde(default)]
    body: Option<HashMap<String, String>>,
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

async fn health_check(servers: Arc<Vec<String>>) {
    loop {
        for ip in servers.iter() {
            let url = "http://".to_owned() + &ip + ":3000" + "/metrics";
            let client = Client::new();

            let response = client.get(url).send().await.expect("metrtics from server");
            if response.status().is_redirection() {
                println!(
                    "server {} gave redirectional error {}",
                    ip,
                    response.status()
                );
            }
            if response.status().is_server_error() {
                println!("server {} gave server error {}", ip, response.status());
            }
            let metrics: Metrics = response.json().await.expect("failed to parse JSON");
            println!("{:?}", metrics);
            if metrics.cpu > 90.0 {
                println!("cpu usage: {}", metrics.cpu);
            }
            if metrics.ram > 90.0 {
                println!("ram usage: {}", metrics.ram);
            }
            if metrics.netspeed[0] < 50.0 {
                println!("download: {}", metrics.netspeed[0]);
            }
            if metrics.netspeed[1] < 50.0 {
                println!("upload: {}", metrics.netspeed[1]);
            }
        }
        sleep(Duration::from_secs(60));
    }
}

async fn api_health_check(api_config: ApiConfig) {
    let time_interval = api_config.check_interval_ms;
    let timeout = api_config.timeout_ms;
    let failure_threshold = api_config.failure_threshold;
    let apis = api_config.apis;
    loop {
        for api in apis.iter() {
            let url = &api.url;
            let action = &api.method;
            let body = &api.body;
            let client = Client::builder()
                .timeout(Duration::from_secs(timeout))
                .build()
                .expect("failed to build the client");
            match action.as_str() {
                "GET" => {
                    let response = client.get(url).send().await;
                    match response {
                        Ok(resp) => {
                            if !resp.status().is_success() {
                                let mut pass = false;
                                for i in 0..failure_threshold {
                                    let response = client.get(url).send().await;
                                    match response {
                                        Ok(resp) => {
                                            if resp.status().is_success() {
                                                pass = true;
                                                break;
                                            }
                                        }
                                        Err(e) if e.is_timeout() => {
                                            println!("timeout for try {}", i)
                                        }
                                        Err(e) => {
                                            println!("error {}", e);
                                        }
                                    }
                                }
                                if pass == false {
                                    println!("response for {} : {}", url, resp.status());
                                }
                            }
                        }
                        Err(_) => {
                            let mut pass = false;
                            for i in 0..failure_threshold {
                                let response = client.get(url).send().await;
                                match response {
                                    Ok(resp) => {
                                        if resp.status().is_success() {
                                            pass = true;
                                            break;
                                        }
                                    }
                                    Err(e) if e.is_timeout() => {
                                        println!("timeout for try {}", i)
                                    }
                                    Err(e) => {
                                        println!("error {}", e);
                                    }
                                }
                            }
                            if pass == false {
                                println!("api {} failed", url);
                            }
                        }
                    }
                }
                "POST" => {
                    let response = client
                        .post(url)
                        .json(&body)
                        .send()
                        .await
                        .expect("api response");
                }
                _ => {}
            }
        }
        sleep(Duration::from_secs(time_interval));
    }
}

pub async fn balance_load() {
    //todo make switch to protocol
    let load_balancer_state = LoadBalancerState::new();
    let address = load_balancer_state.clone().ip + ":3000";
    let servers = load_balancer_state.clone().servers;

    if load_balancer_state
        .features
        .contains(&Features::HealthCheck)
    {
        tokio::spawn(async move {
            health_check(servers).await;
        });
    }
    if load_balancer_state
        .features
        .contains(&Features::ApiHealthCheck)
    {
        let data = fs::read_to_string("api.json").expect("Unable to read file");

        let api_config: ApiConfig = serde_json::from_str(&data).expect("Unable to parse JSON");
        let apis = api_config.apis.clone();

        for api in apis {
            println!("{:?}", api);
        }

        tokio::spawn(async move {
            api_health_check(api_config).await;
        });
    }

    let app = Router::new()
        .route("/{*wildcard}", get(handle_request).post(handle_request))
        .with_state(load_balancer_state);
    let listener = TcpListener::bind(address).await.unwrap();
    println!("loadbalancer is listening...");
    axum::serve(listener, app).await.unwrap();
}

async fn handle_request(
    Path(_path): Path<String>,
    lb: State<LoadBalancerState>,
    req: Request<Body>,
) -> Result<Response<String>, StatusCode> {
    lb.forward_request(req).await
}

use crate::subapps::loadbalancer::ApiConfig;
use log::{error, info, warn};
use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

#[derive(Deserialize, Serialize)]
pub struct ServerData {
    pub server_data: Vec<Payload>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Payload {
    pub cpu: f64,
    pub ram: f64,
    pub netspeed: Vec<f64>,
}

pub async fn health_check(servers: Arc<Vec<String>>) {
    info!("health check spawned");
    loop {
        let mut server_data: Vec<Payload> = vec![];
        for ip in servers.iter() {
            let url = "http://".to_owned() + &ip + ":3001" + "/metrics";
            let client = Client::new();

            match client.get(url).send().await {
                Ok(response) => {
                    if response.status().is_redirection() {
                        info!(
                            "server {} gave redirectional error {}",
                            ip,
                            response.status()
                        );
                    }
                    if response.status().is_server_error() {
                        warn!("server {} gave server error {}", ip, response.status());
                    }
                    let metrics: Payload = response.json().await.expect("failed to parse JSON");

                    if metrics.cpu > 90.0 {
                        warn!("cpu usage: {}", metrics.cpu);
                    }
                    if metrics.ram > 90.0 {
                        warn!("ram usage: {}", metrics.ram);
                    }
                    if metrics.netspeed[0] < 50.0 {
                        warn!("download: {}", metrics.netspeed[0]);
                    }
                    if metrics.netspeed[1] < 50.0 {
                        warn!("upload: {}", metrics.netspeed[1]);
                    }
                    server_data.push(metrics);
                }
                Err(e) => {
                    error!("{}", e);
                }
            };
        }
        let data = ServerData { server_data };
        let url = "http://100.86.175.69:3000/data";
        let client = Client::new();
        match client.post(url).json(&data).send().await {
            Ok(resp) => {
                if !resp.status().is_success() {
                    error!("{}", resp.status());
                }
            }
            Err(err) => {
                warn!("data not sent to tui {:?}", err);
            }
        }
        sleep(Duration::from_secs(1)).await;
    }
}

async fn failed_url_check(failure_threshold: u32, url: &String, client: Client) {
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
                warn!("timeout for try {}", i)
            }
            Err(e) => {
                warn!("{}", e);
            }
        }
    }
    if pass == false {
        error!("{}-> failed", url);
    }
}

pub async fn api_health_check(api_config: ApiConfig) {
    info!("api health check spawned");

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
                                failed_url_check(failure_threshold, url, client).await;
                            }
                        }
                        Err(_) => {
                            if let Ok(_) = client.get(url).send().await {
                                failed_url_check(failure_threshold, url, client).await;
                            } else {
                                failed_url_check(failure_threshold, url, client).await;
                            }
                        }
                    }
                }
                "POST" => {
                    let response = client.post(url).json(&body).send().await;
                    match response {
                        Ok(resp) => {
                            if !resp.status().is_success() {
                                failed_url_check(failure_threshold, url, client).await;
                            }
                        }
                        Err(_) => {
                            if let Ok(_) = client.get(url).send().await {
                                failed_url_check(failure_threshold, url, client).await;
                            } else {
                                failed_url_check(failure_threshold, url, client).await;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        sleep(Duration::from_secs(time_interval)).await;
    }
}

pub async fn load_balancer_connections() {
    info!("load_balancer_connections spawned");
    loop {
        let af_flags = AddressFamilyFlags::IPV4;
        let proto_flags = ProtocolFlags::TCP;

        match get_sockets_info(af_flags, proto_flags) {
            Ok(sockets) => {
                let tcp_count = sockets
                    .into_iter()
                    .filter(|info| matches!(info.protocol_socket_info, ProtocolSocketInfo::Tcp(_)))
                    .count();
                info!("connections: {}", tcp_count);
                if tcp_count > 100 {
                    //env
                    warn!("connections: {}", tcp_count);
                }
            }
            Err(err) => {
                warn!("Failed to get connections: {}", err);
            }
        }
        sleep(Duration::from_secs(60)).await;
    }
}

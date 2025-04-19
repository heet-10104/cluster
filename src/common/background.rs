use crate::subapps::node::Metrics;

use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo};

use std::{sync::Arc, time::Duration};

use crate::subapps::loadbalancer::ApiConfig;
use log::{error, info, warn};
use reqwest::Client;
use tokio::time::sleep;

pub async fn health_check(servers: Arc<Vec<String>>) {
    loop {
        for ip in servers.iter() {
            let url = "http://".to_owned() + &ip + ":3000" + "/metrics";
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
                    let metrics: Metrics = response.json().await.expect("failed to parse JSON");
                    info!("{:?}", metrics);
                    if metrics.cpu > 90.0 {
                        //env
                        warn!("cpu usage: {}", metrics.cpu);
                    }
                    if metrics.ram > 90.0 {
                        //env
                        warn!("ram usage: {}", metrics.ram);
                    }
                    if metrics.netspeed[0] < 50.0 {
                        //env
                        warn!("download: {}", metrics.netspeed[0]);
                    }
                    if metrics.netspeed[1] < 50.0 {
                        //env
                        warn!("upload: {}", metrics.netspeed[1]);
                    }
                }
                Err(e) => {
                    error!("{}", e);
                }
            };
        }
        sleep(Duration::from_secs(60)).await; //env
    }
}

async fn failed_url_check(
    failure_threshold: u32,
    url: &String,
    client: Client,
    resp: reqwest::Response,
) {
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
                warn!("error {}", e);
            }
        }
    }
    if pass == false {
        error!("response for {} : {}", url, resp.status());
    }
}

pub async fn api_health_check(api_config: ApiConfig) {
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
                                failed_url_check(failure_threshold, url, client, resp).await;
                            }
                        }
                        Err(_) => {
                            let resp = client.get(url).send().await;
                            failed_url_check(failure_threshold, url, client, resp.unwrap()).await;
                        }
                    }
                }
                "POST" => {
                    let response = client.post(url).json(&body).send().await;
                    match response {
                        Ok(resp) => {
                            if !resp.status().is_success() {
                                failed_url_check(failure_threshold, url, client, resp).await;
                            }
                        }
                        Err(_) => {
                            let resp = client.get(url).send().await;
                            failed_url_check(failure_threshold, url, client, resp.unwrap()).await;
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
    let af_flags = AddressFamilyFlags::IPV4;
    let proto_flags = ProtocolFlags::TCP;

    match get_sockets_info(af_flags, proto_flags) {
        Ok(sockets) => {
            let tcp_count = sockets
                .into_iter()
                .filter(|info| matches!(info.protocol_socket_info, ProtocolSocketInfo::Tcp(_)))
                .count();
            if tcp_count > 100 {
                //env
                warn!("connections: {}", tcp_count);
            }
        }
        Err(err) => {
            warn!("Failed to get connections: {}", err);
        }
    }
}

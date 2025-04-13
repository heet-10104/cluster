//server listens to loadbalancer for giving the no of connections, metrics
//this works in parallel to the application run by the user
use crate::config::server_config::ServerConfig;
use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sysinfo::{CpuRefreshKind, RefreshKind, System};
use tokio::time::Instant;

#[derive(Serialize, Deserialize, Debug)]
pub struct Metrics {
    pub cpu: f32,
    pub ram: f64,
    pub netspeed: Vec<f64>,
}
//listens to lb and sends the response
pub async fn server_listener() {
    let cfg: ServerConfig = confy::load("server-config", None).expect("Failed to load config");
    let server_ip = cfg.ip;
    let listener = cfg.listener;
    let lbs = cfg.loadbalancer_ip;
    let app = Router::new()
        .route("/connections", get(connections_handler))
        .route("/metrics", get(metrics_handler));

    let address = server_ip + ":3000";
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

    println!("server is listening.....");

    axum::serve(listener, app).await.unwrap();
}

async fn connections_handler() -> impl IntoResponse {
    let count = get_connections();
    (StatusCode::OK, count)
}

async fn metrics_handler() -> impl IntoResponse {
    println!("called metrics handler...");
    let metrics = get_metrics().await;
    (StatusCode::OK, Json(metrics))
}

//gets the no connections with loadbalancers(only src of connection)
//eading from /proc/net/tcp
fn get_connections() -> String {
    let af_flags = AddressFamilyFlags::IPV4;
    let proto_flags = ProtocolFlags::TCP;

    match get_sockets_info(af_flags, proto_flags) {
        Ok(sockets) => {
            let tcp_count = sockets
                .into_iter()
                .filter(|info| matches!(info.protocol_socket_info, ProtocolSocketInfo::Tcp(_)))
                .count();
            format!("connections: {}", tcp_count)
        }
        Err(err) => format!("Failed to get connections: {}", err),
    }
}

//gives cpu consumptions
//ram consumption
//network speed
async fn netspeed_download() -> f64 {
    let url = " http://speedtest.tele2.net/1MB.zip"; // test file
    let client = Client::new();

    let start = Instant::now();
    let response = client
        .get(url)
        .send()
        .await
        .expect("should download the file");
    let bytes = response
        .bytes()
        .await
        .expect("should get the bytes recieved");
    let elapsed = start.elapsed().as_secs_f64();

    let size_in_mb = bytes.len() as f64 / (1024.0 * 1024.0);
    let speed_mbps = size_in_mb / elapsed * 8.0;

    speed_mbps
}

async fn netspeed_upload() -> f64 {
    let client = Client::new();
    let url = "https://httpbin.org/post"; // accepts raw data

    let data_size_bytes = 1 * 1024 * 1024; // 1 MB
    let data = vec![0u8; data_size_bytes];

    let start = Instant::now();
    let response = client
        .post(url)
        .header("Content-Type", "application/octet-stream")
        .body(data)
        .send()
        .await
        .expect("Failed to send request");

    println!("Response: {}", response.status());

    let mut speed_mbps = 0.0;
    if response.status().is_success() {
        let elapsed = start.elapsed().as_secs_f64();
        let speed_mbps = (data_size_bytes as f64 * 8.0) / (elapsed * 1024.0 * 1024.0);
        println!("Upload speed: {:.2} Mbps", speed_mbps);
    } else {
        println!("Server rejected request");
    }

    speed_mbps
}

async fn get_metrics() -> Metrics {
    println!("called get metrics...");
    let mut sys = System::new_all();
    sys.refresh_all();

    let mut s =
        System::new_with_specifics(RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()));
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    s.refresh_cpu_all();
    let mut consumption = 0.0;
    for cpu in s.cpus() {
        consumption += cpu.cpu_usage();
    }
    let total_memory = sys.total_memory() / 1024; // in MB
    let used_memory = sys.used_memory() / 1024; // in MB

    println!("called netspeed d handler...");
    let download = netspeed_download().await;
    // println!("called netspeed u handler...");
    // let upload = netspeed_upload().await;
    let upload = 0.0;

    println!(
        "cpu: {:.2}% | memory: {}MB/{}MB | download: {:?}mbps | upload: {:?}mbps",
        consumption, used_memory, total_memory, download, upload
    );

    let ram: f64 = (used_memory) as f64 / (total_memory) as f64;
    Metrics {
        cpu: consumption,
        ram,
        netspeed: vec![download, upload],
    }
}

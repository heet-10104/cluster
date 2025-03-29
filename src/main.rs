mod common;
use common::background::{dynamic_metrics, Metrics, SystemInfo};
use dialoguer::{theme::ColorfulTheme, Select};
use std::io::{self, Write};
use std::{thread, time::Duration};
use sysinfo::System;

mod config;
mod subapps;
use config::loadbalancer_config::configure_load_balancer;
use config::loadbalancer_config::Protocol;
use config::loadbalancer_config::Features;

enum NodeType {
    LoadBalancer,
    Server,
    MicroServer,
}

enum Config {
    LoadBalancerConfig,
    ServerConfig,
}

impl ToString for NodeType {
    fn to_string(&self) -> String {
        let string = match self {
            NodeType::LoadBalancer => "Load Balancer",
            NodeType::Server => "Server",
            NodeType::MicroServer => "Micro Server",
        };
        string.into()
    }
}

struct ServerConfig {
    ip: String,
    nodes_coneected: Vec<String>,
}


fn background_task() {
    let mut sys = System::new_all();
    sys.refresh_all();

    let mut system_info = SystemInfo::new(&mut sys);
    println!("System Info: {:#?}", system_info);
    let mut metrics = Metrics::new(&mut sys);
    loop {
        dynamic_metrics(&mut sys, &mut metrics, &mut system_info);
        thread::sleep(Duration::from_secs(2));
    }
}

fn main() {
    //thread::spawn(background_task);
    io::stdout().flush().unwrap();

    let node_types = [
        NodeType::LoadBalancer,
        NodeType::Server,
        NodeType::MicroServer,
    ];

    let node_type = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select node type")
        .default(0)
        .items(&node_types)
        .interact()
        .unwrap();

    let node_type = &node_types[node_type];

    match node_type {
        NodeType::LoadBalancer => {
            let protocols = [
                Protocol::RobinRound,
                Protocol::LeastConnections,
                Protocol::LeastResponse,
            ];
            let features = [Features::HealthCheck, Features::ApiHealthCheck];
            configure_load_balancer(&protocols, &features);
        }
        NodeType::Server => {

        }
        NodeType::MicroServer => {}
    }

    loop {
        println!("Main application running...");
        thread::sleep(Duration::from_secs(1));
    }
}

mod common;
use dialoguer::{theme::ColorfulTheme, Select};
use std::io::{self, Write};
use std::{thread, time::Duration};

mod config;
mod subapps;
use config::loadbalancer_config::configure_load_balancer;
use config::loadbalancer_config::Features;
use config::loadbalancer_config::Protocol;
use config::server_config::configure_server;

enum NodeType {
    LoadBalancer,
    Server,
    MicroServer,
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
            configure_server();
        }
        NodeType::MicroServer => {}
    }

    loop {
        println!("Main application running...");
        thread::sleep(Duration::from_secs(1));
    }
}

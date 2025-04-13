mod common;
use dialoguer::Confirm;
use dialoguer::{theme::ColorfulTheme, Select};
use std::io::{self, Write};
use tokio::runtime::Runtime;

mod config;
mod subapps;
use crate::subapps::loadbalancer::balance_load;
use crate::subapps::node::server_listener;
use config::loadbalancer_config::Features;
use config::loadbalancer_config::Protocol;
use config::loadbalancer_config::{configure_load_balancer, LoadBalancerConfig};
use config::server_config::{configure_server, ServerConfig};
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
            let cfg_path: Result<std::path::PathBuf, confy::ConfyError> =
                confy::get_configuration_file_path("load-balancer-config", None);
            if cfg_path.is_ok() {
                println!(
                    "found a file config file for load-balancer at {:?}",
                    cfg_path
                );
                let cfg: LoadBalancerConfig =
                    confy::load("load-balancer-config", None).expect("Failed to load config");
                println!("{:?}", cfg);
                let proceed = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("want to continue with old config?")
                    .default(false)
                    .interact()
                    .unwrap();
                if proceed {
                    let rt = Runtime::new().unwrap();
                    rt.block_on(balance_load());
                }
            }
        }
        NodeType::Server => {
            let cfg_path = confy::get_configuration_file_path("load-balancer-config", None);
            if cfg_path.is_ok() {
                println!("found a file config file for server at {:?}", cfg_path);
                let cfg: ServerConfig =
                    confy::load("server-config", None).expect("Failed to load config");
                println!("{:?}", cfg);
                let proceed = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("want to continue with old config?")
                    .default(false)
                    .interact()
                    .unwrap();
                if proceed {
                    let rt = Runtime::new().unwrap();
                    rt.block_on(server_listener());
                }
            }
        }
        NodeType::MicroServer => {}
    };

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
}

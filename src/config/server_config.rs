use crate::subapps::node::server_listener;
use crate::validator::validate::validate_server_config;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ServerListener {
    HealthCheckListener,
    ApiHealthCheckListener,
}
impl ToString for ServerListener {
    fn to_string(&self) -> String {
        let string = match self {
            ServerListener::HealthCheckListener => "Health Check Listener",
            ServerListener::ApiHealthCheckListener => "Api Health Check Listener",
        };
        string.into()
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct ServerConfig {
    pub ip: String,                    //ip of the machine
    pub listener: Vec<ServerListener>, //what request the server will listen to
    pub loadbalancer_ip: Vec<String>,  // the
}

pub async fn configure_server() {
    let ip: String = Input::new()
        .with_prompt("Enter the IP address of the server(tailscale ip)")
        .interact_text()
        .unwrap();

    let listener = [
        ServerListener::HealthCheckListener,
        ServerListener::ApiHealthCheckListener,
    ];

    let defaults = [true, false];
    let features_selected = dialoguer::MultiSelect::new()
        .with_prompt("Select Listeners")
        .items(&[
            ServerListener::HealthCheckListener,
            ServerListener::ApiHealthCheckListener,
        ])
        .defaults(&defaults[..])
        .interact()
        .unwrap();
    let listener_selected: Vec<ServerListener> = features_selected
        .iter()
        .map(|&index| listener[index].clone())
        .collect();

    let mut next_ip = true;
    let mut loadbalancer_ip: Vec<String> = vec![];
    while next_ip {
        let node_ip: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Connected Loadbalancers")
            .validate_with(|input: &String| -> Result<(), &str> {
                let numbers: Vec<&str> = input.split('.').collect();

                if numbers.len() != 4 {
                    return Err("IPv4 address must have exactly 4 octets");
                }

                for num in numbers {
                    match num.parse::<u8>() {
                        Ok(_) => continue,
                        Err(_) => return Err("Each octet must be a number between 0 and 255"),
                    }
                }

                Ok(())
            })
            .interact_text()
            .unwrap();
        loadbalancer_ip.push(node_ip);

        let add_more = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Add more nodes?")
            .default(false)
            .interact()
            .unwrap();
        if !add_more {
            next_ip = false;
        }
    }

    let config = ServerConfig {
        ip,
        listener: listener_selected,
        loadbalancer_ip,
    };
    println!("Server Config: {:#?}", config);

    if !validate_server_config(&config) {
        println!("Invalid configuration. Please check the IP addresses and try again.");
        return;
    }
    confy::store("server-config", None, config).expect("Failed to store config");
    server_listener().await;
}

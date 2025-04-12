use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use indicatif::{ProgressBar, ProgressStyle};
use std::{process::Command, thread, time::Duration};
use tokio::runtime::Runtime;

use crate::subapps::node::server_listener;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum ServerListener {
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
struct ServerConfig {
    ip: String,                    //ip of the machine
    listener: Vec<ServerListener>, //what request the server will listen to
    loadbalancer_ip: Vec<String>,       // the 
}

pub fn configure_server() {
    let ip: String = Input::new()
        .with_prompt("Enter the IP address of the server(tailscale ip)")
        .interact_text()
        .unwrap();

    let ports: Vec<u16> = vec![];

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
                .with_prompt("Connected Server IPv4")
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
        loadbalancer_ip
    };
    println!("Server Config: {:?}", config);

    if !validate_config(&config) {
        println!("Invalid configuration. Please check the IP addresses and try again.");
        return;
    }
    confy::store("server-config", None, config).expect("Failed to store config");
    let rt = Runtime::new().unwrap();
    rt.block_on(server_listener());
}

fn validate_config(config: &ServerConfig) -> bool {
    let ip = &config.ip;
    let mut valid = true;
    if is_ip_live(&ip) {
        println!("server {} is live", ip);
    } else {
        println!("server {} is not reachable", ip);
        let _ = !valid;
    }

    let loadblancers: Vec<String> = config.loadbalancer_ip.iter().map(|ip| ip.clone()).collect();
    let bar = ProgressBar::new(loadblancers.len() as u64);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} Checking {msg}")
            .unwrap(),
    );
    for ip in &loadblancers {
        bar.set_message(ip.clone());
        if is_ip_live(ip) {
            println!("{} is live", ip);
        } else {
            valid = false;
            println!("{} is not reachable", ip);
        }
        bar.inc(1);
        thread::sleep(Duration::from_millis(500));
    }
    bar.finish_with_message("Validation complete.");
    valid

}

fn is_ip_live(ip: &str) -> bool {
    let output = Command::new("ping").arg("-c").arg("1").arg(ip).output();

    match output {
        Ok(result) => result.status.success(),
        Err(_) => false,
    }
}

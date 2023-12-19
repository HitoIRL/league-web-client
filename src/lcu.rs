#![allow(dead_code)] // because we don't use unsubscribe and update right now
// i dont think we will ever use update in the future
// so might delete it in next commits

use std::{collections::HashMap, process::Command};

use log::{debug, error};
use reqwest::{Method, Client, header::{ACCEPT, CONTENT_TYPE, AUTHORIZATION}};

pub enum EventKind {
    Subscribe = 5,
    Unsubscribe = 6,
    Update = 8,
}

pub fn get_cmd_data() -> HashMap<String, String> {
    let cmd = Command::new("cmd")
        .args(&["/C", "wmic PROCESS WHERE name='LeagueClientUx.exe' GET commandline"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&cmd.stdout);

    let mut hashmap = HashMap::new();
    let args: Vec<_> = stdout.split("--").collect();

    for item in args {
        let value: Vec<_> = item.split("=").collect();

        if value.len() < 2 {
            continue;
        }

        hashmap.insert(
            value[0].to_string(),
            value[1].replace("\"", "").trim().to_string(),
        );
    }

    hashmap
}

pub async fn send_request(uri: &str, token: &str, method: Method) {
    debug!("Sending a request: {uri:?} {method:?}");

    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .no_proxy()
        .build()
        .unwrap();

    let request = client
        .request(method, uri)
        .header(ACCEPT, "application/json, text/plain")
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, format!("Basic {token}"))
        .send()
        .await;

    match request {
        Ok(response) => {
            let body = response.text().await.unwrap();
            debug!("Received response: {body:?}");
        }
        Err(why) => error!("Failed to send a request: {why}")
    }
}

pub fn serialize_event(kind: EventKind, name: &str) -> String {
    let delimiter_count = name.matches("/").count();
    let event_str = match delimiter_count {
        0 => name.to_string(),
        _ => "OnJsonApiEvent".to_string() + &name.replacen("/", "_", delimiter_count),
    };

    format!("[{:?},\"{event_str}\"]", kind as u8)
}

mod logger;
mod lcu;

use std::collections::HashMap;

use base64::{engine::general_purpose, Engine};
use futures_util::{StreamExt, SinkExt};
use http::{Request, header::{AUTHORIZATION, HOST, UPGRADE, CONNECTION, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION}};
use lcu::EventKind;
use log::{info, error, debug};
use native_tls::TlsConnector;
use reqwest::Method;
use serde_json::Value;
use tokio_tungstenite::tungstenite::Message;

type EventBody = (EventKind, String);

#[tokio::main]
async fn main() {
    if let Err(why) = logger::setup() {
        eprintln!("Failed to setup logger: {why}, you won't see any logs!");
    }

    let cmd_data = lcu::get_cmd_data();

    let url = format!("127.0.0.1:{}", cmd_data["app-port"]);
    let token = general_purpose::STANDARD.encode(format!("riot:{}", cmd_data["remoting-auth-token"]));

    info!("Connected to LCU! url: {url}, token: {token}");

    // establishing websocket connection
    let connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .build()
        .unwrap();
    let connector = tokio_tungstenite::Connector::NativeTls(connector);

    let request = Request::builder()
        .uri(format!("wss://{}", url))
        .method("GET")
        .header(AUTHORIZATION, format!("Basic {token}"))
        .header(HOST, "127.0.0.1")
        .header(UPGRADE, "websocket")
        .header(CONNECTION, "upgrade")
        .header(SEC_WEBSOCKET_KEY, "lcu")
        .header(SEC_WEBSOCKET_VERSION, "13")
        .body(())
        .unwrap();
    let (socket, _) = tokio_tungstenite::connect_async_tls_with_config(request, None, false, Some(connector))
        .await
        .unwrap();

    // spliting the socket into sender and receiver
    let (mut writer, mut reader) = socket.split();
    let (sender, mut receiver) = tokio::sync::mpsc::channel::<EventBody>(32);

    // callbacks
    let receive_task = tokio::spawn(async move {
        while let Some(msg) = reader.next().await {
            let msg = match msg {
                Ok(msg) => msg,
                Err(why) => {
                    error!("Error while receiving message: {why}");
                    continue;
                }
            };

            if (msg.is_text() || msg.is_binary()) && !msg.is_empty() {
                let (_kind, name, body): (u8, String, HashMap<String, Value>) = serde_json::from_str(&msg.to_string()).unwrap();

                debug!("Received event {name}: {body:?}");

                match body.get("data") {
                    Some(data) => {
                        match data.get("phase") {
                            Some(phase) => if phase == "ReadyCheck" {
                                info!("Accepting ready check...");
                                let uri = format!("https://{url}/lol-matchmaking/v1/ready-check/accept");
                                lcu::send_request(&uri, &token, Method::POST).await;
                            }
                            None => {}
                        }
                    }
                    None => {}
                }
            }
        }
    });

    let send_task = tokio::spawn(async move {
        while let Some((kind, name)) = receiver.recv().await {
            let serialized = lcu::serialize_event(kind, &name);

            debug!("Sending a message: {serialized}");
            if let Err(why) = writer.send(Message::Text(serialized)).await {
                error!("Error while sending message: {why}");
            }
        }
    });

    // subscribe to gameflow events
    sender.send((EventKind::Subscribe, String::from("/lol-gameflow/v1/session"))).await.unwrap();

    receive_task.await.unwrap();
    send_task.await.unwrap();
}

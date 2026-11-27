use super::http::LcuClient;
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, http::HeaderValue, Message};
use tokio_tungstenite::{connect_async_tls_with_config, Connector};

/// 只订阅用得到的几个事件，而不是全量 `OnJsonApiEvent`，以减少流量和解析开销。
pub const EVENTS: [&str; 3] = [
    "OnJsonApiEvent_lol-gameflow_v1_gameflow-phase",
    "OnJsonApiEvent_lol-champ-select_v1_session",
    "OnJsonApiEvent_lol-matchmaking_v1_ready-check",
];

pub struct LcuEvent {
    pub uri: String,
    pub event_type: String,
    pub data: Value,
}

/// 连接 WAMP WebSocket，事件经 channel 送出；连接断开时返回。
pub async fn run(port: u16, client: &LcuClient, tx: mpsc::Sender<LcuEvent>) -> Result<()> {
    let mut req = format!("wss://127.0.0.1:{port}/").into_client_request()?;
    req.headers_mut()
        .insert("Authorization", HeaderValue::from_str(client.auth_header())?);
    let tls = native_tls::TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .build()?;
    let (mut ws, _) =
        connect_async_tls_with_config(req, None, true, Some(Connector::NativeTls(tls))).await?;

    for e in EVENTS {
        ws.send(Message::Text(format!("[5,\"{e}\"]").into())).await?;
    }

    while let Some(msg) = ws.next().await {
        let text = match msg? {
            Message::Text(t) => t,
            Message::Close(_) => break,
            _ => continue,
        };
        // 消息格式：[8, "<事件名>", { "data": ..., "eventType": "Update", "uri": "/..." }]
        let Ok(Value::Array(mut arr)) = serde_json::from_str::<Value>(text.as_str()) else {
            continue;
        };
        if arr.len() < 3 || arr[0] != 8 {
            continue;
        }
        let mut payload = arr.swap_remove(2);
        let ev = LcuEvent {
            uri: payload["uri"].as_str().unwrap_or_default().to_string(),
            event_type: payload["eventType"].as_str().unwrap_or_default().to_string(),
            data: payload["data"].take(),
        };
        if tx.send(ev).await.is_err() {
            break;
        }
    }
    Ok(())
}

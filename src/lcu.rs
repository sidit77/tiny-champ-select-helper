use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use anyhow::{anyhow, Result, Context};
use async_native_tls::{Certificate, TlsConnector};
use async_std::{fs, task};
use async_tungstenite::async_std::{connect_async_with_tls_connector, ConnectStream};
use async_tungstenite::tungstenite::Message;
use async_tungstenite::WebSocketStream;
use error_tools::IgnoreResult;
use futures::{SinkExt, StreamExt};
use http::Request;
use notify::{RecommendedWatcher, RecursiveMode, Watcher, EventKind};
use surf::{Client, Config};
use surf::http::auth::BasicAuth;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use serde_repr::{Serialize_repr, Deserialize_repr};
use crate::Asset;

#[derive(Debug, Clone, PartialEq)]
pub struct RiotLockFile {
    pub process: String,
    pub pid: u32,
    pub port: u32,
    pub password: String,
    pub protocol: String,
    pub username: String,
    pub address: String,
}


impl RiotLockFile {

    pub async fn read<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            wait_for(path).await?;
            task::sleep(Duration::from_secs(5)).await
        }

        let contents = fs::read_to_string(path).await?;

        let pieces: Vec<&str> = contents.split(':').collect();

        let username = "riot".to_string();
        let address = "127.0.0.1".to_string();
        let process = pieces[0].to_string();
        let pid = pieces[1].parse()?;
        let port = pieces[2].parse()?;
        let password = pieces[3].to_string();
        let protocol = pieces[4].to_string();

        Ok(Self {
            process,
            pid,
            port,
            password,
            protocol,
            username,
            address
        })
    }

    pub async fn connect(&self) -> Result<(Client, LcuWebSocket)> {
        let auth = BasicAuth::new(&self.username, &self.password);
        let cert = Certificate::from_pem(Asset::get("riotgames.pem").context("can't find cert")?.data.as_ref())?;

        let client = Config::new()
            .set_base_url(format!("{}://{}:{}", self.protocol, self.address, self.port).parse()?)
            .set_tls_config(Some(Arc::new(TlsConnector::new()
                .add_root_certificate(cert.clone()))))
            .add_header(auth.name(), auth.value()).map_err(|e| anyhow!(e))?
            .try_into()?;

        let (socket, _) = connect_async_with_tls_connector(
            Request::builder()
                .uri(format!("wss://{}:{}", self.address, self.port))
                .header(auth.name().as_str(), auth.value().as_str())
                .body(())
                .unwrap(),
            Some(TlsConnector::new()
                .add_root_certificate(cert))
        ).await?;

        Ok((client, LcuWebSocket {
            socket
        }))
    }

}

async fn wait_for<P: AsRef<Path>>(path: P) -> notify::Result<()> {
    let path = path.as_ref();
    let (tx, mut rx) = async_std::channel::unbounded();
    let mut watcher = RecommendedWatcher::new(move |event: notify::Result<notify::Event>| tx.try_send(event).ignore())?;
    watcher.watch(path.parent().unwrap(), RecursiveMode::NonRecursive)?;
    log::trace!("Started to watch for {:?}", path);
    while let Some(event) = rx.next().await {
        match event {
            Ok(event) => {
                log::trace!("Got event: {:?}", event);
                if matches!(event.kind, EventKind::Create(_)) && event.paths.iter().any(|p| p == path) {
                    log::trace!("Found expected file");
                    return Ok(())
                }
            }
            Err(e) => return Err(e)
        }
    }
    return Err(notify::Error::generic("Watched stopped unexpectedly"))
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u32)]
enum ActionCode {
    Subscribe = 5,
    Unsubscribe = 6,
    Event = 8
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
struct Action(ActionCode, String);

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
struct Event(ActionCode, String, EventArgs);

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
struct EventArgs {
    data: Value,
    #[serde(rename = "eventType")]
    event_type: String,
    uri: String
}

#[derive(Debug)]
pub struct LcuWebSocket {
    socket: WebSocketStream<ConnectStream>
}

impl LcuWebSocket {

    async fn send(&mut self, action: &Action) -> Result<()> {
        self.socket.send(serde_json::to_string(action)?.into()).await?;
        Ok(())
    }

    pub async fn subscribe(&mut self, endpoint: impl AsRef<str>) -> Result<()> {
        self.send(&Action(ActionCode::Subscribe,
                          format!("OnJsonApiEvent{}", endpoint.as_ref()).replace('/', "_"))).await
    }

    //pub async fn unsubscribe(&mut self, endpoint: impl AsRef<str>) -> Result<()> {
    //    self.send(&Action(ActionCode::Unsubscribe,
    //                      format!("OnJsonApiEvent{}", endpoint.as_ref()).replace("/", "_"))).await
    //}

    pub async fn read(&mut self) -> Result<Option<(String, Value)>> {
        loop {
            let msg = self.socket.next().await.transpose()?;

            match msg {
                Some(Message::Text(str)) if !str.is_empty() => {
                    let event = serde_json::from_str::<Event>(&str)?;
                    return Ok(Some((event.2.uri, event.2.data)))
                },
                None => return Ok(None),
                _ => continue
            }

        }

    }

}
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod lcu;

use std::fmt::Debug;
use anyhow::{Result};
use async_broadcast::{InactiveReceiver, Receiver, TrySendError};
use async_std::{fs, task};
use async_std::prelude::FutureExt as AsyncStdFutureExt;
use async_std::sync::{Mutex, Arc};
use async_std::task::JoinHandle;
use error_tools::IgnoreResult;
use futures::{FutureExt, select, StreamExt};
use log::LevelFilter;
use tide::{Request, Response};
use tide::http::mime;
use tide_websockets::{Message, WebSocket};
use serde::{Serialize};
use serde_json::Value;
use surf::Client;
use tray_item::TrayItem;
use crate::lcu::RiotLockFile;

#[derive(Debug, Clone, Serialize)]
struct BasicInfo {
    server: String,
    username: String
}

impl BasicInfo {
    async fn load_from(client: &Client) -> Self {
        Self {
            server: client.get("/riotclient/region-locale").recv_json::<Value>().await.unwrap()["region"].as_str().unwrap().to_lowercase(),
            username: client.get("/lol-summoner/v1/current-summoner").recv_json::<Value>().await.unwrap()["displayName"].as_str().unwrap().into()
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize)]
enum ClientState {
    Closed,
    Idle,
    ChampSelect,
    InGame
}

impl From<&str> for ClientState {
    fn from(s: &str) -> Self {
        match s {
            "ChampSelect" => Self::ChampSelect,
            "InProgress" => Self::InGame,
            _ => Self::Idle
        }
    }
}

impl ClientState {
    async fn load_from(client: &Client) -> Self {
        Self::from(client.get("/lol-gameflow/v1/gameflow-phase").recv_json::<Value>().await.unwrap().as_str().unwrap())
    }
}

impl Default for ClientState {
    fn default() -> Self {
        Self::Closed
    }
}

#[derive(Debug, Clone, Default, Serialize)]
struct ClientStatus {
    state: ClientState,
    info: Option<BasicInfo>,
    additional_info: Option<Vec<String>>
}

impl ClientStatus {

    async fn update(&mut self, client: &Client, state: ClientState) {
        self.state = state;
        match state {
            ClientState::Closed => self.info = None,
            _ => if self.info.is_none() {
                self.info = Some(BasicInfo::load_from(client).await)
            }
        }
        match state {
            ClientState::ChampSelect => if self.additional_info.is_none() {
                let player_ids = client.get("/lol-champ-select/v1/session").recv_json::<Value>().await
                    .unwrap()["myTeam"]
                    .as_array().unwrap().iter()
                    .map(|je| je["summonerId"].as_u64().unwrap())
                    .filter(|id| *id > 0)
                    .collect::<Vec<_>>();

                let player_names = client.get(format!("/lol-summoner/v2/summoner-names?ids={:?}", player_ids)).recv_json::<Value>()
                    .await.unwrap()
                    .as_array().unwrap().iter()
                    .map(|je| je["displayName"].as_str().unwrap().to_string())
                    .collect::<Vec<_>>();

                self.additional_info = Some(player_names)
            }
            _ => self.additional_info = None
        }
    }

    async fn load_from(client: &Client) -> Self {
        let mut result = Self::default();
        result.update(client, ClientState::load_from(client).await).await;
        result
    }

}

async fn run(address: &str) -> Result<()> {
    let (mut sender, receiver) = async_broadcast::broadcast(10);

    let lockfile = RiotLockFile::read("D:\\Games\\Riot Games\\League of Legends\\lockfile")?;

    let _handler = async_std::task::spawn(async move {
        sender.set_overflow(true);
        let (client, mut socket) = lockfile.connect().await.unwrap();

        let mut status = ClientStatus::load_from(&client).await;
        sender.try_broadcast(status.clone()).ignore();

        socket.subscribe("/lol-gameflow/v1/gameflow-phase").await.unwrap();

        loop {
            match socket.read().await {
                Ok(Some((uri, json))) if uri == "/lol-gameflow/v1/gameflow-phase" => {
                    match json.as_str() {
                        Some(state) => {
                            let state = ClientState::from(state);
                            if state != status.state {
                                status.update(&client, state).await;
                                match sender.try_broadcast(status.clone()) {
                                    Ok(_) | Err(TrySendError::Inactive(_)) => {},
                                    Err(TrySendError::Closed(_)) => break,
                                    Err(TrySendError::Full(_)) => unreachable!()
                                }
                            }
                        },
                        None => log::warn!("Invalid data")
                    }
                },
                Ok(Some(tuple)) => log::warn!("Unknown event: {:?}", tuple),
                Ok(None) => break,
                Err(err) => log::warn!("{}", err)
            }
        }

        status.update(&client, ClientState::Closed).await;
        sender.try_broadcast(status.clone()).ignore();
    });


    /*
    let _handler = async_std::task::spawn(async move {
        sender.set_overflow(true);
        let mut lines = BufReader::new(stdin()).lines().fuse();
        loop {
            match lines.next().await {
                Some(Ok(line)) => match sender.try_broadcast(line) {
                    Ok(_) | Err(TrySendError::Inactive(_)) => {},
                    Err(TrySendError::Closed(_)) => break,
                    Err(TrySendError::Full(_)) => unreachable!()
                },
                Some(Err(e)) => log::error!("{}", e),
                None => break
            }
        }
    });
     */

    let mut app = tide::with_state(ReceiveWrapper::new(receiver));
    app.at("/").get(|_| async move {
        Ok(Response::builder(200)
            .body( if cfg!(debug_assertions) {
                fs::read_to_string("assets/index.html").await.unwrap()
            } else {
                include_str!("../assets/index.html").to_string()
            })
            .content_type(mime::HTML)
            .build())
    });
    app.at("/socket").get(WebSocket::new(|req: Request<ReceiveWrapper<ClientStatus>>, mut stream| async move {
        let (state, mut receiver) = req.state().subscribe().await;
        stream.send_string(serde_json::to_string(&state)?).await?;
        loop {
            select! {
                msg = stream.next().fuse() => match msg {
                    Some(Ok(Message::Close(_))) => {}
                    Some(msg) => log::info!("Got unexpected message: {:?}", msg),
                    None => break
                },
                msg = receiver.next().fuse() => match msg {
                    Some(msg) => stream.send_string(serde_json::to_string(&msg)?).await?,
                    None => break
                }
            }
        }
        Ok(())
    }));
    app.listen(address).await?;
    Ok(())
}

#[derive(Clone)]
struct ReceiveWrapper<T> {
    receiver: InactiveReceiver<T>,
    last_value: Arc<Mutex<T>>,
    _handle: Arc<JoinHandle<()>>
}

impl <T> ReceiveWrapper<T>
    where T: Default + Clone + Send + Sync + 'static
{
    fn new(receiver: Receiver<T>) -> Self {
        Self::new_with_default(receiver, T::default())
    }
}

impl<T> ReceiveWrapper<T>
    where T: Clone + Send + Sync + 'static
{

    fn new_with_default(receiver: Receiver<T>, default_value: impl Into<T>) -> Self {
        let last_value = Arc::new(Mutex::new(default_value.into()));
        let _handle = Arc::new({
            let last_value = last_value.clone();
            let mut receiver = receiver.clone();
            task::spawn(async move {
                while let Some(val) = receiver.next().await {
                    let mut x = last_value.lock_arc().await;
                    *x = val;
                }
            })
        });
        Self {
            receiver: receiver.deactivate(),
            last_value,
            _handle
        }
    }

    async fn subscribe(&self) -> (T, Receiver<T>) {
        let receiver = self.receiver.activate_cloned();
        let value = self.last_value.lock_arc().await.clone();
        (value, receiver)
    }

}

fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .filter(Some("tungstenite::protocol"), LevelFilter::Info)
        .filter(Some("tungstenite::handshake::client"), LevelFilter::Info)
        .filter(Some("tide::log::middleware"), LevelFilter::Warn)
        .format_timestamp(None)
        //.format_target(false)
        .parse_default_env()
        .init();

    let address = "127.0.0.1:43257";
    let open = move || webbrowser::open(&format!("http://{}", address)).unwrap();

    let quitter = async_ctrlc::CtrlC::new()?;

    let (sender, mut receiver) = async_std::channel::bounded(2);
    let mut tray = TrayItem::new("Tray Example", "favicon").unwrap();
    tray.add_menu_item("Open", open).unwrap();
    tray.add_menu_item("Quit", move || {
        sender.try_send(()).unwrap();
    }).unwrap();
    let quitter = quitter.race(receiver.next().map(|r|r.unwrap()));

    open();
    task::block_on(run(address).race(quitter.map(|_ | Ok(()))))
}

/*
#[async_std::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        //.filter(Some("tungstenite"), LevelFilter::Trace)
        .format_timestamp(None)
        //.format_target(false)
        .init();



    let lockfile = RiotLockFile::read("D:\\Games\\Riot Games\\League of Legends\\lockfile")?;
    let (client, mut socket) = lockfile.connect().await?;

    println!("{:#}", serde_json::from_str::<Value>(&client.get("/lol-summoner/v1/current-summoner").recv_string().await.unwrap())?);

    println!("{:#}", serde_json::from_str::<Value>(&client.get("/riotclient/region-locale").recv_string().await.unwrap())?);

    println!("{:#}", serde_json::from_str::<Value>(&client.get("/lol-gameflow/v1/gameflow-phase").recv_string().await.unwrap())?);

    //println!("{:#}", serde_json::from_str::<Value>(&client.get("/help").recv_string().await.unwrap())?);

    socket.subscribe("/lol-gameflow/v1/gameflow-phase").await?;

    loop {
        match socket.read().await {
            Ok((uri, json)) if uri == "/lol-gameflow/v1/gameflow-phase" => {
                match json.as_str() {
                    Some("ChampSelect") => {
                        log::debug!("Entering Champ Select");
                        let player_ids = serde_json::from_str::<Value>(
                            &client.get("/lol-champ-select/v1/session").recv_string().await.unwrap())?["myTeam"]
                            .as_array().unwrap().iter()
                            .map(|je| je["summonerId"].as_u64().unwrap())
                            .filter(|id| *id > 0)
                            .collect::<Vec<_>>();

                        let player_names = serde_json::from_str::<Value>(
                            &client.get(format!("/lol-summoner/v2/summoner-names?ids={:?}", player_ids)).recv_string().await.unwrap())?
                            .as_array().unwrap().iter()
                            .map(|je| je["displayName"].as_str().unwrap().to_string())
                            .collect::<Vec<_>>();

                        println!("{:?}", player_names);
                    }
                    v => log::warn!("Unknown gamestate: {:?}", v)
                }
            },
            Ok(tuple) => log::warn!("Unknown event: {:?}", tuple),
            Err(err) => log::warn!("{}", err)
        }
    }
}
*/

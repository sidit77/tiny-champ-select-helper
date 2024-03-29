#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod lcu;
mod config;
mod client_state;
mod util;

use std::path::Path;
use std::time::Duration;
use anyhow::{Result};
use async_broadcast::TrySendError;
use async_std::{task};
use async_std::prelude::FutureExt as AsyncStdFutureExt;
use error_tools::IgnoreResult;
use futures::{FutureExt, StreamExt};
use futures::future::Either;
use log::LevelFilter;
use tide::{Body, Redirect, Request, Response};
use tide::http::{mime, Mime};
use tide_websockets::{Message, WebSocket};
use tray_item::TrayItem;
use rust_embed::{EmbeddedFile, RustEmbed};
use surf::StatusCode;
use crate::client_state::{ClientState, ClientStatus};
use crate::config::Config;
use crate::lcu::RiotLockFile;
use crate::util::ReceiveWrapper;

#[derive(RustEmbed)]
#[folder = "assets/"]
pub struct Asset;

async fn run(config: &Config) -> Result<()> {
    let (mut sender, receiver) = async_broadcast::broadcast(10);

    let lockfile_path = Path::new(&config.client_path).join("lockfile");

    let _handler = async_std::task::spawn(async move {
        sender.set_overflow(true);

        'outer: loop {
            let lockfile = RiotLockFile::read(&lockfile_path).await.unwrap();
            log::info!("found lockfile");
            let (client, mut socket) = lockfile.connect().await.unwrap();

            let mut status = loop {
                match ClientStatus::load_from(&client).await {
                    Ok(res) => break res,
                    Err(err) => log::error!("Error occurred. Retrying...\n{}", err)
                };
                task::sleep(Duration::from_millis(500)).await;
            };
            sender.try_broadcast(status.clone()).ignore();

            socket.subscribe("/lol-gameflow/v1/gameflow-phase").await.unwrap();

            loop {
                match socket.read().await {
                    Ok(Some((uri, json))) if uri == "/lol-gameflow/v1/gameflow-phase" => {
                        match json.as_str() {
                            Some(state) => {
                                let state = ClientState::from(state);
                                if state != status.state {
                                    //retry(Duration::from_millis(500), || status.update(&client, state)).await;
                                    loop {
                                        match status.update(&client, state).await {
                                            Ok(res) => break res,
                                            Err(err) => log::error!("Error occurred. Retrying...\n{}", err)
                                        };
                                        task::sleep(Duration::from_millis(500)).await;
                                    }
                                    match sender.try_broadcast(status.clone()) {
                                        Ok(_) | Err(TrySendError::Inactive(_)) => {},
                                        Err(TrySendError::Closed(_)) => break 'outer,
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

            status.update(&client, ClientState::Closed).await.unwrap();
            sender.try_broadcast(status.clone()).ignore();
        }

    });

    let mut app = tide::with_state(ReceiveWrapper::new(receiver));
    app.at("*").get(|req: tide::Request<ReceiveWrapper<ClientStatus>> | async move {
        let path = req.url().path().trim_start_matches('/');
        log::debug!("trying to load {}", path);
        let asset: Option<EmbeddedFile> = Asset::get(path);

        match asset {
            None => Ok(Response::new(StatusCode::NotFound)),
            Some(file) => {
                let mime = Mime::sniff(file.data.as_ref())
                    .ok()
                    .or_else(|| Path::new(path)
                        .extension()
                        .map(|p| p.to_str())
                        .flatten()
                        .and_then(Mime::from_extension))
                    .unwrap_or(mime::BYTE_STREAM);
                log::debug!("detected mime type: {}", mime);
                Ok(Response::builder(StatusCode::Ok)
                    .body(Body::from_bytes(file.data.into()))
                    .content_type(mime)
                    .build())
            }
        }
    });
    app.at("/").get(Redirect::permanent("/index.html"));
    app.at("/socket").get(WebSocket::new(|req: Request<ReceiveWrapper<ClientStatus>>, mut stream| async move {
        let (state, mut receiver) = req.state().subscribe().await;
        stream.send_string(serde_json::to_string(&state)?).await?;
        loop {
            match futures::future::select(stream.next(), receiver.next()).await {
                Either::Left((msg, _)) => match msg {
                    Some(Ok(Message::Close(_))) => {}
                    Some(msg) => log::info!("Got unexpected message: {:?}", msg),
                    None => break
                }
                Either::Right((msg, _)) => match msg {
                    Some(msg) => stream.send_string(serde_json::to_string(&msg)?).await?,
                    None => break
                }
            }
        }
        Ok(())
    }));
    app.listen(&config.server_url).await?;
    Ok(())
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

    let config = Config::initialize()?;
    let open = {
        let addrs = config.server_url.clone();
        move || webbrowser::open(&format!("http://{}", addrs)).unwrap()
    };

    let quitter = async_ctrlc::CtrlC::new()?;

    let (sender, mut receiver) = async_std::channel::bounded(2);
    let mut tray = TrayItem::new("Tiny Champ Select Helper", "favicon").unwrap();
    tray.add_menu_item("Open", open.clone()).unwrap();
    tray.add_menu_item("Quit", move || {
        sender.try_send(()).unwrap();
    }).unwrap();
    let quitter = quitter.race(receiver.next().map(|r|r.unwrap()));

    open();
    task::block_on(run(&config).race(quitter.map(|_ | Ok(()))))
}
//mod lcu;

use anyhow::{Result};
use async_broadcast::{InactiveReceiver, TrySendError};
use async_std::{fs};
use async_std::io::{BufReader, stdin};
use async_std::io::prelude::BufReadExt;
use futures::{FutureExt, select, StreamExt};
use log::LevelFilter;
use tide::{Request, Response};
use tide::http::mime;
use tide_websockets::{Message, WebSocket};

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .filter(Some("tungstenite::protocol"), LevelFilter::Info)
        .filter(Some("tide::log::middleware"), LevelFilter::Warn)
        .format_timestamp(None)
        //.format_target(false)
        .parse_default_env()
        .init();

    let (mut sender, receiver) = async_broadcast::broadcast(10);

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

    let mut app = tide::with_state(receiver.deactivate());
    app.at("/").get(|_| async move {
        Ok(Response::builder(200)
            .body(fs::read_to_string("assets/index.html").await.unwrap())
            .content_type(mime::HTML)
            .build())
    });
    app.at("/socket").get(WebSocket::new(|req: Request<InactiveReceiver<String>>, mut stream| async move {
        let mut receiver = req.state().activate_cloned();
        loop {
            select! {
                msg = stream.next().fuse() => match msg {
                    Some(Ok(Message::Close(_))) => {}
                    Some(msg) => log::info!("Got unexpected message: {:?}", msg),
                    None => break
                },
                msg = receiver.next().fuse() => match msg {
                    Some(msg) => stream.send_string(msg).await?,
                    None => break
                }
            }
        }
        log::info!("Connection closed");
        Ok(())
    }));
    app.listen("127.0.0.1:8080").await?;
    Ok(())
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

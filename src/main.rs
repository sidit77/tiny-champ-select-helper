mod lcu;

use std::time::Instant;
use anyhow::{Result};
use async_std::fs;
use futures::StreamExt;
use log::LevelFilter;
use serde_json::Value;
use tide::{Request, Response};
use tide::http::mime;
use tide_websockets::{Message, WebSocket};
use crate::lcu::RiotLockFile;

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        //.filter(Some("tungstenite"), LevelFilter::Trace)
        .filter(Some("tide::log::middleware"), LevelFilter::Warn)
        .format_timestamp(None)
        //.format_target(false)
        .parse_default_env()
        .init();

    let mut app = tide::with_state(Instant::now());
    app.at("/").get(main_site);
    app.at("/time").get(time);
    app.at("/socket").get(WebSocket::new(|_req, mut stream| async move {
        while let Some(Ok(Message::Text(input))) = stream.next().await {
            let output: String = input.chars().rev().collect();

            stream
                .send_string(format!("{} | {}", &input, &output))
                .await?;
        }
        log::info!("Connection closed");
        Ok(())
    }));
    app.listen("127.0.0.1:8080").await?;
    Ok(())
}

async fn main_site(_: Request<Instant>) -> tide::Result<Response> {
    Ok(Response::builder(200)
        .body(fs::read_to_string("assets/index.html").await.unwrap())
        .content_type(mime::HTML)
        .build())
}

async fn time(req: Request<Instant>) -> tide::Result<Response> {
    Ok(Response::builder(200)
        .body(req.state().elapsed().as_secs().to_string())
        .content_type(mime::PLAIN)
        .build())
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

/*
use horrorshow::html;
use tide::{Request, Response};
use tide::http::{mime};
use tide::prelude::*;

#[derive(Debug, Deserialize)]
struct Animal {
    name: String,
    legs: u8,
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    let mut app = tide::new();
    app.at("/").get(main_site);
    app.at("/orders/shoes").post(order_shoes);
    app.listen("127.0.0.1:8080").await?;
    Ok(())
}

async fn main_site(_: Request<()>) -> tide::Result<Response> {

    let body = html! {
        style { : "* { font-family: sans-serif}" }
        body {
            : "Hello World"
        }
    };

    Ok(Response::builder(200)
        .body(format!("{}", body))
        .content_type(mime::HTML)
        .build())
}

async fn order_shoes(mut req: Request<()>) -> tide::Result {
    let Animal { name, legs } = req.body_json().await?;
    Ok(format!("Hello, {}! I've put in an order for {} shoes", name, legs).into())
}
*/
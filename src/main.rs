mod lcu;

use anyhow::{Result};
use log::LevelFilter;
use serde_json::Value;
use surf::{Client};
use surf::http::auth::BasicAuth;
use crate::lcu::RiotLockFile;

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .format_timestamp(None)
        .format_target(false)
        .init();

    let lockfile = RiotLockFile::read("D:\\Games\\Riot Games\\League of Legends\\lockfile")?;
    let client = Client::try_from(lockfile)?;

    println!("{:#}", serde_json::from_str::<Value>(&client.get("/lol-summoner/v1/current-summoner").recv_string().await.unwrap())?);

    println!("{:#}", serde_json::from_str::<Value>(&client.get("/riotclient/region-locale").recv_string().await.unwrap())?);

    println!("{:#}", serde_json::from_str::<Value>(&client.get("/lol-gameflow/v1/gameflow-phase").recv_string().await.unwrap())?);
    Ok(())
}



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
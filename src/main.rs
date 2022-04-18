use std::fs;
use std::path::Path;
use std::sync::Arc;
use base64::encode;
use anyhow::{anyhow, Result};
use async_native_tls::{Certificate, TlsConnector};
use surf::{Client, Config, Url};
use surf::http::auth::BasicAuth;

#[async_std::main]
async fn main() -> Result<()> {
    let logfile = RiotLockFile::read("D:\\Games\\Riot Games\\League of Legends\\lockfile")?;
    println!("{:?}", logfile);
    let url = format!("{}://{}:{}", logfile.protocol, logfile.address, logfile.port).parse::<Url>()?;
    println!("Url: {}", url);
    let auth = BasicAuth::new(logfile.username, logfile.password);
    println!("Auth: {:?}", auth);
    let tls_config = TlsConnector::new()
        .add_root_certificate(Certificate::from_pem(include_bytes!("../assets/riotgames.pem"))?);

    let client: Client = Config::new()
        .set_base_url(url)
        .set_tls_config(Some(Arc::new(tls_config)))
        .add_header(auth.name(), auth.value()).map_err(|e| anyhow!(e))?
        .try_into()?;

    let req = client.get("/lol-summoner/v1/current-summoner");
    //println!("Req: {}", req.build().url());

    println!("{:?}", req.recv_string().await);
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct RiotLockFile {
    pub process: String,
    pub pid: u32,
    pub port: u32,
    pub password: String,
    pub protocol: String,
    pub username: String,
    pub address: String,
    pub b64_auth: String,
}

impl RiotLockFile {

    pub fn read<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = fs::read_to_string(path)?;

        let pieces: Vec<&str> = contents.split(":").collect();

        let username = "riot".to_string();
        let address = "127.0.0.1".to_string();
        let process = pieces[0].to_string();
        let pid = pieces[1].parse()?;
        let port = pieces[2].parse()?;
        let password = pieces[3].to_string();
        let protocol = pieces[4].to_string();
        let b64_auth = encode(format!("{}:{}", username, password).as_bytes());

        Ok(Self {
            process,
            pid,
            port,
            password,
            protocol,
            username,
            address,
            b64_auth,
        })
    }

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
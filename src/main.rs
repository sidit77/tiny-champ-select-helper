use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::sync::mpsc::channel;
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Result, Watcher};
use std::time::Duration;
use base64::encode;

fn main() -> Result<()> {
    let (tx, rx) = channel();
    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher = notify::watcher(tx, Duration::from_secs(5))?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch("D:\\Games\\Riot Games\\League of Legends", RecursiveMode::NonRecursive)?;

    loop {
        match rx.recv() {
            Ok(DebouncedEvent::Create(path) | DebouncedEvent::Write(path))
                if path.ends_with("lockfile")=> println!("Connect\n{:?}", RiotLockFile::read(path)),
            Ok(DebouncedEvent::NoticeRemove(path) | DebouncedEvent::Write(path))
                if path.ends_with("lockfile")=> println!("Disconnect"),
            Err(e) => println!("watch error: {:?}", e),
            _ => {}
        }
    }

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

    pub fn read<P: AsRef<Path>>(path: P) -> Self {
        let contents = fs::read_to_string(path).unwrap();

        let pieces: Vec<&str> = contents.split(":").collect();

        let username = "riot".to_string();
        let address = "127.0.0.1".to_string();
        let process = pieces[0].to_string();
        let pid = pieces[1].parse().unwrap();
        let port = pieces[2].parse().unwrap();
        let password = pieces[3].to_string();
        let protocol = pieces[4].to_string();
        let b64_auth = encode(format!("{}:{}", username, password).as_bytes());

        Self {
            process,
            pid,
            port,
            password,
            protocol,
            username,
            address,
            b64_auth,
        }
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
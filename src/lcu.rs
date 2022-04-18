use std::fs;
use std::path::Path;
use std::sync::Arc;
use anyhow::{anyhow, Result};
use async_native_tls::{Certificate, TlsConnector};
use surf::{Client, Config};
use crate::BasicAuth;

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

}

impl TryFrom<RiotLockFile> for Client {
    type Error = anyhow::Error;

    fn try_from(lockfile: RiotLockFile) -> std::result::Result<Self, Self::Error> {
        let url = format!("{}://{}:{}", lockfile.protocol, lockfile.address, lockfile.port).parse()?;
        let auth = BasicAuth::new(lockfile.username, lockfile.password);
        let tls_config = TlsConnector::new()
            .add_root_certificate(Certificate::from_pem(include_bytes!("../assets/riotgames.pem"))?);

        let client = Config::new()
            .set_base_url(url)
            .set_tls_config(Some(Arc::new(tls_config)))
            .add_header(auth.name(), auth.value()).map_err(|e| anyhow!(e))?
            .try_into()?;

        Ok(client)
    }
}
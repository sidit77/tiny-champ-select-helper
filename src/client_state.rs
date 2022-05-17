use serde_json::Value;
use surf::Client;
use serde::Serialize;
use anyhow::Result;
use error_tools::{OptionToError, WrapError};

#[derive(Debug, Clone, Serialize)]
pub struct BasicInfo {
    pub server: String,
    pub username: String
}

impl BasicInfo {
    pub async fn load_from(client: &Client) -> Result<Self> {
        Ok(Self {
            server: client
                .get("/riotclient/region-locale")
                .recv_json::<Value>()
                .await.wrapped()?
                .get("region").err()?
                .as_str().err()?
                .to_lowercase(),
            username: client
                .get("/lol-summoner/v1/current-summoner")
                .recv_json::<Value>()
                .await.wrapped()?
                .get("displayName").err()?
                .as_str().err()?
                .into()
        })
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize)]
pub enum ClientState {
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
    pub async fn load_from(client: &Client) -> Result<Self> {
        Ok(Self::from(client
            .get("/lol-gameflow/v1/gameflow-phase")
            .recv_json::<Value>()
            .await.wrapped()?
            .as_str().err()?))
    }
}

impl Default for ClientState {
    fn default() -> Self {
        Self::Closed
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ClientStatus {
    pub state: ClientState,
    pub info: Option<BasicInfo>,
    pub additional_info: Option<Vec<String>>
}

impl ClientStatus {

    pub async fn update(&mut self, client: &Client, state: ClientState) -> Result<()> {
        match state {
            ClientState::Closed => self.info = None,
            _ => if self.info.is_none() {
                self.info = Some(BasicInfo::load_from(client).await?)
            }
        }
        match state {
            ClientState::ChampSelect => if self.additional_info.is_none() {
                let player_ids = client
                    .get("/lol-champ-select/v1/session")
                    .recv_json::<Value>()
                    .await.wrapped()?
                    .get("myTeam").err()?
                    .as_array().err()?
                    .iter()
                    .filter_map(|je| je
                        .get("summonerId")
                        .and_then(|id| id.as_u64()))
                    .filter(|id| *id > 0)
                    .collect::<Vec<_>>();

                let player_names = client
                    .get(format!("/lol-summoner/v2/summoner-names?ids={:?}", player_ids))
                    .recv_json::<Value>()
                    .await.wrapped()?
                    .as_array().err()?
                    .iter()
                    .filter_map(|je| je
                        .get("displayName")
                        .and_then(|name| name.as_str())
                        .map(|name| name.to_string()))
                    .collect::<Vec<_>>();

                self.additional_info = Some(player_names)
            }
            _ => self.additional_info = None
        }
        self.state = state;
        Ok(())
    }

    pub async fn load_from(client: &Client) -> Result<Self> {
        let mut result = Self::default();
        result.update(client, ClientState::load_from(client).await?).await?;
        Ok(result)
    }

}
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct MessageArgs<'a> {
    pub channel: &'a str,
    pub message: &'a str,
}

#[derive(Clone, Debug)]
pub struct MessageData {
    pub id: usize,
    pub channel: String,
    pub user: String,
    pub content: String,
    pub is_self: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetworkData {
    pub name: String,
    pub channels: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionSettings {
    pub server: String,
    pub nick: String,
    pub real_name: Option<String>,
    pub nickserv_password: Option<String>,
    pub nickserv_account: Option<String>,
}

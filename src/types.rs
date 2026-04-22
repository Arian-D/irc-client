use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct MessageArgs<'a> {
    pub user: &'a str,
    pub message: &'a str,
}

#[derive(Clone, Debug)]
pub struct MessageData {
    pub id: usize,
    pub user: String,
    pub content: String,
    pub is_self: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetworkData {
    pub name: String,
    pub channels: Vec<String>,
}

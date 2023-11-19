use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub runner: Runner,
    pub daemon: Daemon,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Runner {
    pub shell: String,
    pub args: Vec<String>,
    pub log_path: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Daemon {
    pub interval: u64,
    pub kind: String,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(dead_code)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub avatar_url: Option<String>,
}

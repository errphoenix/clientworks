use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub enum Payload {
    Chat { message: String },
    Disconnect { reason: Option<String> },
    Connect { latency: u64 },
}

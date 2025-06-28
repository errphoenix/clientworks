use crate::api::Server;

pub enum ConnectionStatus {
    Connected,
    Disconnected,
}

pub struct ConnectionHandle {
    pub server: Server,
    pub status: ConnectionStatus,
    
}
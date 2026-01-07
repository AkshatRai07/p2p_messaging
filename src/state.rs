use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};    
use std::time::Instant;

pub type PeerMap = Arc<Mutex<HashMap<SocketAddr, Instant>>>;

pub fn init_peers() -> PeerMap {
    Arc::new(Mutex::new(HashMap::new()))
}

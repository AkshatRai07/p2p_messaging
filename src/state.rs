use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

pub type PeerMap = Arc<Mutex<HashSet<SocketAddr>>>;

pub fn init_peers() -> PeerMap {
    Arc::new(Mutex::new(HashSet::new()))
}

use std::net::UdpSocket;
use std::thread;
use std::time::Duration;
use crate::state::PeerMap;

const BROADCAST_ADDR: &str = "255.255.255.255";
const PROTOCOL_MSG: &[u8] = b"HELLO_P2P";

pub fn start_background_tasks(socket: UdpSocket, peers: PeerMap, port: u16) {
    
    let socket_listener = socket.try_clone().expect("failed to clone into listener");
    let socket_broadcaster = socket.try_clone().expect("failed to clone into broadcaster");
    
    thread::spawn(move || {
        let mut buffer = [0u8; 1024];
        loop {
            match socket_listener.recv_from(&mut buffer) {
                Ok((size, source_addr)) => {
                    if &buffer[..size] == PROTOCOL_MSG {
                        let mut p = peers.lock().unwrap();
                        p.insert(source_addr);
                    }
                }
                Err(_) => { /* Ignore errors in background to avoid spamming UI */ }
            }
        }
    });
    
    thread::spawn(move || {
        loop {
            let target = format!("{}:{}", BROADCAST_ADDR, port);
            let _ = socket_broadcaster.send_to(PROTOCOL_MSG, &target);
            thread::sleep(Duration::from_secs(5));
        }
    });
}

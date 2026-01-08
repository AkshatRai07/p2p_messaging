use std::net::{UdpSocket, TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::mpsc::Sender;
use crate::state::PeerMap;

const BROADCAST_ADDR: &str = "255.255.255.255";
const PROTOCOL_MSG: &[u8] = b"HELLO_P2P";

const PEER_TIMEOUT: Duration = Duration::from_secs(15);
const BROADCAST_INTERVAL: Duration = Duration::from_secs(5);

pub fn start_background_tasks(
    socket: UdpSocket,
    peers: PeerMap,
    port: u16,
    conn_sender: Sender<TcpStream>
) {
    
    let socket_listener = socket.try_clone().expect("failed to clone into listener");
    let socket_broadcaster = socket.try_clone().expect("failed to clone into broadcaster");
    let peers_cleanup = peers.clone();
    
    thread::spawn(move || {
        let mut buffer = [0u8; 1024];
        loop {
            match socket_listener.recv_from(&mut buffer) {
                Ok((size, source_addr)) => {
                    if &buffer[..size] == PROTOCOL_MSG {
                        let mut p = peers.lock().unwrap();
                        p.insert(source_addr, Instant::now());
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
            thread::sleep(BROADCAST_INTERVAL);
        }
    });

    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(2));
            let mut p = peers_cleanup.lock().unwrap();
            p.retain(|_, last_seen| last_seen.elapsed() < PEER_TIMEOUT);
        }
    });

    thread::spawn(move || {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
            .expect("Could not bind TCP listener");
        
        for stream in listener.incoming() {
            match stream {
                Ok(s) => {
                    let _ = conn_sender.send(s);
                }
                Err(e) => eprintln!("Connection failed: {}", e),
            }
        }
    });
}

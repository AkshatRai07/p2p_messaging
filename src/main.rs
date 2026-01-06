use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use std::io::Result;
use std::thread;
use std::time::Duration;

const PORT: u16 = 8888;
const BROADCAST_ADDR: &str = "255.255.255.255";
const PROTOCOL_MSG: &[u8] = b"HELLO_P2P";

fn main() -> Result<()> {
    let socket: UdpSocket = UdpSocket::bind(format!("0.0.0.0:{}", PORT)).expect("couldn't bind to address");

    socket.set_broadcast(true).expect("set_broadcast call failed");

    let known_peers = Arc::new(Mutex::new(HashSet::new()));

    println!("Node started on port {}. Listening for peers...", PORT);

    let socket_listener = socket.try_clone().expect("failed to clone into broadcaster thread");
    let peers_listener = Arc::clone(&known_peers);

    thread::spawn(move || {
        let mut buffer = [0u8; 1024];
        loop {
            match socket_listener.recv_from(&mut buffer) {
                Ok((size, source_addr)) => {
                    if &buffer[..size] == PROTOCOL_MSG {
                        let mut peers = peers_listener.lock().unwrap();

                        if peers.insert(source_addr) {
                            println!("\n[Listener] New Peer Discovered: {}", source_addr);
                        }
                    }
                }
                Err(e) => eprintln!("[Listener] Error receiving data: {}", e),
            }
        }
    });

    let socket_broadcaster = socket.try_clone().expect("failed to clone into sender thread");

    thread::spawn(move || {
        loop {
            let target = format!("{}:{}", BROADCAST_ADDR, PORT);
            if let Err(e) = socket_broadcaster.send_to(PROTOCOL_MSG, &target) {
                eprintln!("[Broadcaster] Failed to send packet: {}", e);
            }
            
            thread::sleep(Duration::from_secs(5));
        }
    });

    loop {
        thread::sleep(Duration::from_secs(10));
        let peers = known_peers.lock().unwrap();
        println!("---------------------------------");
        println!("Current Known Peers ({} total):", peers.len());
        for peer in peers.iter() {
            println!(" - {}", peer);
        }
        println!("---------------------------------");
    }
}

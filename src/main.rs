mod state;
mod network;
mod chat;

use std::io::{self, Write};
use std::net::UdpSocket;
use std::sync::mpsc;
use std::time::Duration;

use crossterm::{
    execute,
    terminal::{Clear, ClearType, SetTitle, enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    event::{self, KeyCode, Event},
    cursor,
};
use colored::*;

const PORT: u16 = 3000;

fn main() -> std::io::Result<()> {
    
    execute!(io::stdout(), SetTitle("Sandesh P2P"))?;
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", PORT)).expect("couldn't bind");
    socket.set_broadcast(true).expect("set_broadcast failed");

    let known_peers = state::init_peers();
    let (tx, rx) = mpsc::channel();
    network::start_background_tasks(socket, known_peers.clone(), PORT, tx);

    clear_screen();
    print_banner();

    loop {
        
        if let Ok(stream) = rx.try_recv() {
            chat::handle_incoming_request(stream)?;
            print_prompt(); 
        }

        print_prompt();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let command_line = input.trim();

        let parts: Vec<&str> = command_line.split_whitespace().collect();
        if parts.is_empty() { continue; }
        let command = parts[0];
        let args = &parts[1..];

        match command {
            "find" => {
                monitor_peers(&known_peers)?;
            }
            "find-quick" => {
                let peers = known_peers.lock().unwrap();
                println!("{}", "--- Known Peers ---".yellow());
                if peers.is_empty() {
                    println!("No peers found yet.");
                } else {
                    for (peer, _) in peers.iter() {
                        println!(" - {}", peer);
                    }
                }
                println!("{}", "-------------------".yellow());
            }
            "connect" => {
                if args.is_empty() {
                    println!("Usage: connect <IP:PORT> (e.g., connect 192.168.1.5:8888)");
                } else {
                    // If user forgot port, append default
                    let target = if args[0].contains(':') {
                        args[0].to_string()
                    } else {
                        format!("{}:{}", args[0], PORT)
                    };
                    
                    chat::initiate_connection(&target)?;
                }
            }
            "cls" | "clear" => {
                clear_screen();
                print_banner();
            }
            "help" => {
                println!("  find              - Live monitor of active peers (Raw Mode)");
                println!("  find-quick        - List currently known peers immediately");
                println!("  connect <ip:port> - Request chat with a peer");
                println!("  cls | clear       - Clear screen");
                println!("  exit              - Close the application");
            }
            "exit" => {
                println!("Shutting down Sandesh...");
                break;
            }
            "" => {} 
            _ => println!("Unknown command."),
        }
    }
    Ok(())
}

fn print_prompt() {
    print!("{}", "\nSANDESH >> ".green().bold());
    io::stdout().flush().unwrap();
}

fn monitor_peers(shared_peers: &state::PeerMap) -> io::Result<()> {
    enable_raw_mode()?; 
    let mut stdout = io::stdout();

    execute!(stdout, EnterAlternateScreen, cursor::Show)?;
    execute!(stdout, cursor::MoveTo(0, 0))?;
    println!("(Press 'q' or 'Esc' to return to menu)\r");
    println!("{}\r", "Scanning for Peers...".yellow());
    println!("{}\r", "---------------------------------".dimmed());

    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    _ => {}
                }
            }
        }

        let current_peers = shared_peers.lock().unwrap();

        execute!(
            stdout, 
            cursor::MoveTo(0, 3), 
            Clear(ClearType::FromCursorDown)
        )?;

        if current_peers.is_empty() {
             println!("{}\r", "Waiting for signals...".italic().dimmed());
        } else {
            let mut sorted_peers: Vec<_> = current_peers.keys().collect();
            sorted_peers.sort();

            for peer in sorted_peers {
                println!("{} {}\r", "•".green(), peer);
            }
        }
        
        drop(current_peers);
        stdout.flush()?;
    }
    
    execute!(stdout, LeaveAlternateScreen, cursor::Show)?;
    disable_raw_mode()?;
    Ok(())
}

fn clear_screen() {
    execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0)).unwrap();
}

fn print_banner() {
    let banner = r#"
   _____  ___    _   ______  ___________ __  __
  / ___/ /   |  / | / / __ \/ ____/ ___// / / /
  \__ \ / /| | /  |/ / / / / __/  \__ \/ /_/ / 
 ___/ // ___ |/ /|  / /_/ / /___ ___/ / __  /  
/____//_/  |_/_/ |_/_____/_____//____/_/ /_/   
                                               
    "#;
    println!("{}", banner.cyan().bold());
    println!("Welcome to {}. v0.1.0", "SANDESH".yellow());
    println!("Type '{}' to start.\n", "help".italic());
}

mod state;
mod network;

use std::io::{self, Write};
use std::net::UdpSocket;
use std::time::Duration;
use std::collections::HashSet;
use std::thread;

use crossterm::{
    execute,
    terminal::{Clear, ClearType, SetTitle, enable_raw_mode, disable_raw_mode},
    event::{self, KeyCode, Event},
    cursor,
};
use colored::*;

const PORT: u16 = 8888;

fn main() -> std::io::Result<()> {
    
    execute!(io::stdout(), SetTitle("Sandesh P2P"))?;
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", PORT)).expect("couldn't bind");
    socket.set_broadcast(true).expect("set_broadcast failed");

    let known_peers = state::init_peers();
    network::start_background_tasks(socket, known_peers.clone(), PORT);

    clear_screen();
    print_banner();

    loop {
        
        print!("{}", "\nSANDESH >> ".green().bold());
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let command = input.trim();

        match command {
            "find" => {
                monitor_peers(&known_peers)?;
                clear_screen();
                print_banner(); 
            }
            "cls" | "clear" => {
                clear_screen();
                print_banner();
            }
            "help" => {
                println!("  find  - Live monitor of active peers");
                println!("  exit  - Close the application");
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

fn monitor_peers(shared_peers: &state::PeerMap) -> io::Result<()> {
    
    enable_raw_mode()?; 
    let mut stdout = io::stdout();
    
    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
    
    println!("(Press 'q' or 'Esc' to return to menu)\r");
    println!("{}\r", "Scanning for Peers...".yellow());
    println!("{}\r", "---------------------------------".dimmed());

    let mut displayed_peers = HashSet::new();

    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        break;
                    }
                    _ => {}
                }
            }
        }

        let current_peers = shared_peers.lock().unwrap();
        
        for peer in current_peers.iter() {
            if !displayed_peers.contains(peer) {
                println!("{} {}\r", "+".green(), peer);
                displayed_peers.insert(*peer);
            }
        }
        drop(current_peers);
        thread::sleep(Duration::from_millis(50));
    }
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

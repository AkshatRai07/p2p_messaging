mod state;
mod network;
mod chat;
mod crypto;

use std::io::{self, Write};
use std::net::UdpSocket;
use std::sync::mpsc;
use std::time::Duration;

use crossterm::{
    cursor,
    execute,
    terminal::{Clear, ClearType, SetTitle, enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    event::{self, KeyCode, Event},
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

    enable_raw_mode()?;
    print_prompt("");

    let mut input_buffer = String::new();
    
    let mut command_history: Vec<String> = Vec::new();
    let mut history_index: usize = 0;

    loop {
        if let Ok(stream) = rx.try_recv() {
            disable_raw_mode()?; 
            chat::handle_incoming_request(stream)?;
            enable_raw_mode()?;
            print_prompt(&input_buffer); 
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char(c) => {
                        input_buffer.push(c);
                        print!("{}", c);
                        io::stdout().flush()?;
                    }
                    KeyCode::Backspace => {
                        if input_buffer.pop().is_some() {
                            print!("\x08 \x08"); 
                            io::stdout().flush()?;
                        }
                    }
                    KeyCode::Up => {
                        if !command_history.is_empty() && history_index > 0 {
                            history_index -= 1;
                            input_buffer = command_history[history_index].clone();
                            print_prompt_clean(&input_buffer);
                        }
                    }
                    KeyCode::Down => {
                        if history_index < command_history.len() {
                            history_index += 1;
                            
                            if history_index == command_history.len() {
                                input_buffer.clear();
                            } else {
                                input_buffer = command_history[history_index].clone();
                            }
                            print_prompt_clean(&input_buffer);
                        }
                    }
                    KeyCode::Enter => {
                        println!("\r"); 
                        let command_line = input_buffer.trim().to_string();
                        
                        if !command_line.is_empty() {
                            command_history.push(command_line.clone());
                        }
                        
                        history_index = command_history.len();
                        
                        input_buffer.clear();
                        
                        disable_raw_mode()?; 
                        handle_command(&command_line, &known_peers)?;
                        enable_raw_mode()?;

                        print_prompt("");
                    }
                    _ => {}
                }
            }
        }
    }
}

fn print_prompt_clean(text: &str) {
    print!("\r");
    execute!(io::stdout(), crossterm::terminal::Clear(crossterm::terminal::ClearType::UntilNewLine)).unwrap();
    print!("{} {}", "SANDESH >> ".green().bold(), text);
    io::stdout().flush().unwrap();
}

fn handle_command(input: &str, known_peers: &state::PeerMap) -> io::Result<()> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() { return Ok(()); }
    
    let command = parts[0];
    let args = &parts[1..];

    match command {
        "find" => {
            monitor_peers(known_peers)?;
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
                println!("Usage: connect <IP:PORT>");
            } else {
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
            println!("  find              - Live monitor of active peers");
            println!("  find-quick        - List known peers");
            println!("  connect <ip:port> - Request chat");
            println!("  cls | clear       - Clear screen");
            println!("  exit              - Close application");
        }
        "exit" => {
            println!("Shutting down...");
            std::process::exit(0);
        }
        _ => println!("Unknown command."),
    }
    Ok(())
}

fn print_prompt(current_input: &str) {
    print!("\r{} {}", "\nSANDESH >> ".green().bold(), current_input);
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

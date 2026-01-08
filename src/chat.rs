use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use colored::*;

// Protocol Signals
const SIGNAL_ACCEPT: u8 = b'Y';
const SIGNAL_REJECT: u8 = b'N';

/// The Receiver side: Prompts user to accept/reject
pub fn handle_incoming_request(mut stream: TcpStream) -> io::Result<()> {
    let peer_addr = stream.peer_addr()?;
    
    // Clear current line and ask
    print!("\r\n{} {} {} (y/n)? ", "Incoming connection from".yellow(), peer_addr, "Accept".bold());
    io::stdout().flush()?;

    // Simple blocking input for the Y/N decision
    let mut response = String::new();
    io::stdin().read_line(&mut response)?;

    if response.trim().eq_ignore_ascii_case("y") {
        // Send Accept Signal
        stream.write_all(&[SIGNAL_ACCEPT])?;
        enter_chat_window(stream)?;
    } else {
        // Send Reject Signal
        let _ = stream.write_all(&[SIGNAL_REJECT]); // Ignore error if they disconnected
        println!("{}", "Connection rejected.".red());
    }
    Ok(())
}

/// The Sender side: Initiates connection and waits for response
pub fn initiate_connection(target_ip: &str) -> io::Result<()> {
    println!("{}", format!("Connecting to {}...", target_ip).yellow());
    
    match TcpStream::connect(target_ip) {
        Ok(mut stream) => {
            // Set a timeout for the handshake so we don't hang forever
            stream.set_read_timeout(Some(Duration::from_secs(10)))?;

            println!("Waiting for peer to accept...");
            
            let mut buffer = [0u8; 1];
            match stream.read_exact(&mut buffer) {
                Ok(_) => {
                    if buffer[0] == SIGNAL_ACCEPT {
                        stream.set_read_timeout(None)?; // Remove timeout for chat
                        enter_chat_window(stream)?;
                    } else {
                        println!("{}", "Connection was rejected by peer.".red());
                    }
                }
                Err(_) => println!("{}", "Connection timed out or peer disconnected.".red()),
            }
        }
        Err(e) => println!("{} {}", "Failed to connect:".red(), e),
    }
    Ok(())
}

/// The actual Chat UI (Alternate Screen)
fn enter_chat_window(mut stream: TcpStream) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::MoveTo(0, 0), Clear(ClearType::All))?;

    // Non-blocking stream so we can read and type at the same time
    stream.set_nonblocking(true)?;

    println!("Connected to {}.\r", stream.peer_addr()?);
    println!("(Press 'Esc' to disconnect)\r");
    println!("---------------------------------\r");

    loop {
        // 1. Check for User Input (Poll)
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => break, // Exit chat
                    // Here you would add logic to capture chars and send them
                    // For now, we just exit on Esc as requested
                    _ => {}
                }
            }
        }

        // 2. Check for Incoming Data (Non-blocking Read)
        let mut buffer = [0u8; 1024];
        match stream.read(&mut buffer) {
            Ok(0) => {
                // Connection closed by peer
                println!("\r\nPeer disconnected.\r");
                break;
            }
            Ok(_n) => {
                // Logic to display received message goes here
                // let msg = String::from_utf8_lossy(&buffer[..n]);
                // print!("{}\r", msg);
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                // No data yet, continue loop
            }
            Err(_) => break, // Real error
        }
    }

    // Cleanup
    execute!(stdout, LeaveAlternateScreen)?;
    disable_raw_mode()?;
    println!("{}", "Session ended.".yellow());
    Ok(())
}

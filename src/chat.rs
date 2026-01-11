use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, size, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    style::{Print, Color, SetForegroundColor},
};
use colored::*;
use crate::crypto;
use chacha20poly1305::{ChaCha20Poly1305, KeyInit};

const SIGNAL_ACCEPT: u8 = b'Y';
const SIGNAL_REJECT: u8 = b'N';

pub fn handle_incoming_request(mut stream: TcpStream) -> io::Result<()> {
    let peer_addr = stream.peer_addr()?;

    print!("\r\n{} {} {} (y/n)? ", "Incoming connection from".yellow(), peer_addr, "Accept".bold());
    io::stdout().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;

    if response.trim().eq_ignore_ascii_case("y") {
        stream.write_all(&[SIGNAL_ACCEPT])?;
        enter_chat_window(stream)?;
    } else {
        let _ = stream.write_all(&[SIGNAL_REJECT]); 
        println!("{}", "Connection rejected.".red());
    }
    Ok(())
}

pub fn initiate_connection(target_ip: &str) -> io::Result<()> {
    println!("{}", format!("Connecting to {}...", target_ip).yellow());
    
    match TcpStream::connect(target_ip) {
        Ok(mut stream) => {
            stream.set_read_timeout(Some(Duration::from_secs(10)))?;
            println!("Waiting for peer to accept...");
            
            let mut buffer = [0u8; 1];
            match stream.read_exact(&mut buffer) {
                Ok(_) => {
                    if buffer[0] == SIGNAL_ACCEPT {
                        stream.set_read_timeout(None)?; 
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

fn enter_chat_window(mut stream: TcpStream) -> io::Result<()> {
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Clear(ClearType::All))?;
    println!("Performing Secure Handshake...");

    let shared_secret = match crypto::perform_handshake(&stream) {
        Ok(s) => s,
        Err(e) => {
            println!("Handshake failed: {}", e);
            std::thread::sleep(Duration::from_secs(2));
            return Ok(());
        }
    };

    let cipher = ChaCha20Poly1305::new_from_slice(&shared_secret)
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "Invalid Key"))?;

    stream.set_nonblocking(true)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Clear(ClearType::All))?;

    stream.set_nonblocking(true)?;

    let peer_addr = stream.peer_addr()?.to_string();
    let mut input_buffer = String::new();
    let mut messages: Vec<String> = Vec::new();
    let mut scroll_offset: usize = 0; 
    
    messages.push(format!("Connected to {}.", peer_addr));
    messages.push("End-to-End Encrypted.".to_string());
    messages.push("Press 'Esc' to disconnect.".to_string());
    messages.push("---------------------------------".to_string());

    draw_ui(&mut stdout, &messages, &input_buffer, scroll_offset)?;

    loop {
        let mut needs_redraw = false;

        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => break,
                    KeyCode::Enter => {
                     if !input_buffer.is_empty() {
                         if let Err(e) = crypto::encrypt_and_send(&mut stream, &cipher, &input_buffer) {
                             messages.push(format!("Error: {}", e));
                         } else {
                             messages.push(format!("{} >> {}", " [You]".green(), input_buffer));
                             input_buffer.clear();
                             scroll_offset = 0;
                         }
                         needs_redraw = true;
                     }
                 }
                    KeyCode::Char(c) => {
                        input_buffer.push(c);
                        needs_redraw = true;
                    }
                    KeyCode::Backspace => {
                        input_buffer.pop();
                        needs_redraw = true;
                    }
                    KeyCode::PageUp | KeyCode::Up => {
                        let (_cols, rows) = size()?;
                        let view_height = (rows as usize).saturating_sub(2);
                        let max_scroll = messages.len().saturating_sub(view_height);
                        
                        if scroll_offset < max_scroll {
                            scroll_offset += 1;
                            needs_redraw = true;
                        }
                    }
                    KeyCode::PageDown | KeyCode::Down => {
                        if scroll_offset > 0 {
                            scroll_offset -= 1;
                            needs_redraw = true;
                        }
                    }
                    _ => {}
                }
            }
        }

        match crypto::receive_and_decrypt(&mut stream, &cipher) {
            Ok(msg) => {
                if !msg.is_empty() {
                    messages.push(format!("{} >> {}", "[They]".cyan(), msg));
                    needs_redraw = true;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                // No data waiting
            }
            Err(_) => {
                messages.push("Peer disconnected.".red().to_string());
                draw_ui(&mut stdout, &messages, &input_buffer, scroll_offset)?;
                std::thread::sleep(Duration::from_secs(2));
                break;
            }
        }

        if needs_redraw {
            draw_ui(&mut stdout, &messages, &input_buffer, scroll_offset)?;
        }
    }

    execute!(stdout, LeaveAlternateScreen)?;
    disable_raw_mode()?;
    println!("{}", "Session ended.".yellow());
    Ok(())
}

fn draw_ui(
    stdout: &mut io::Stdout, 
    messages: &[String], 
    input_buffer: &str, 
    scroll_offset: usize
) -> io::Result<()> {
    let (cols, rows) = size()?;
    execute!(stdout, Clear(ClearType::All))?;

    let available_lines = (rows as usize).saturating_sub(2);
    
    let total_msgs = messages.len();
    let end_index = total_msgs.saturating_sub(scroll_offset);
    let start_index = end_index.saturating_sub(available_lines);

    let slice = if start_index < messages.len() && end_index <= messages.len() {
        &messages[start_index..end_index]
    } else {
        &[] 
    };

    execute!(stdout, cursor::MoveTo(0, 0))?;
    for msg in slice {
        print!("{}\r\n", msg);
    }

    let separator_row = rows.saturating_sub(2);
    execute!(stdout, cursor::MoveTo(0, separator_row))?;
    let line = "-".repeat(cols as usize);
    execute!(stdout, SetForegroundColor(Color::DarkGrey), Print(line), SetForegroundColor(Color::Reset))?;

    let input_row = rows.saturating_sub(1);
    execute!(stdout, cursor::MoveTo(0, input_row))?;
    print!("{} {}", ">>".green().bold(), input_buffer);

    io::stdout().flush()?;
    Ok(())
}

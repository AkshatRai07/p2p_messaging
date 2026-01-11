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
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Clear(ClearType::All))?;

    stream.set_nonblocking(true)?;

    let peer_addr = stream.peer_addr()?.to_string();
    let mut input_buffer = String::new();
    let mut messages: Vec<String> = Vec::new();
    let mut scroll_offset: usize = 0; 
    
    messages.push(format!("Connected to {}.", peer_addr));
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
                            if let Err(e) = stream.write_all(input_buffer.as_bytes()) {
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

        let mut buffer = [0u8; 512];
        match stream.read(&mut buffer) {
            Ok(0) => {
                messages.push("Peer disconnected.".red().to_string());
                draw_ui(&mut stdout, &messages, &input_buffer, scroll_offset)?;
                std::thread::sleep(Duration::from_secs(2));
                break;
            }
            Ok(n) => {
                let s = String::from_utf8_lossy(&buffer[..n]);
                let clean_msg = s.trim();
                if !clean_msg.is_empty() {
                    messages.push(format!("{} >> {}", "[They]".cyan(), clean_msg));
                    needs_redraw = true;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(_) => {
                messages.push("Connection Error.".red().to_string());
                needs_redraw = true;
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

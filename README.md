# Sandesh

**Sandesh** (Hindi for *Message*) is a lightweight, terminal-based Peer-to-Peer (P2P) chat application written in Rust. It features automatic local network discovery, end-to-end encryption, and a clean command-line interface.

## Features

* **Serverless Architecture:** No central database or relay server. Communication is direct between peers.
* **Automatic Discovery:** Uses UDP broadcasting to automatically find other users on the local network (LAN).
* **End-to-End Encryption:** All chat messages are encrypted using **ChaCha20Poly1305** with ephemeral **X25519** key exchange.
* **Terminal UI:** Rich TUI experience with command history, scrollable chat logs, and raw mode input using `crossterm`.
* **Thread-Safe State:** Handles background network tasks (heartbeats, cleanup, listening) concurrently without freezing the UI.

## Installation

### Prerequisites

* [Rust and Cargo](https://rustup.rs/) installed.

### Option 1: Install via Cargo

If the crate is available, you can install the binary directly:

```bash
cargo install sandesh
```

*Make sure your `~/.cargo/bin` is in your system PATH.*

### Option 2: Build from Source

1. Clone the repository:
```bash
git clone https://github.com/yourusername/sandesh-p2p.git
cd sandesh-p2p
```

2. Run the application locally:
```bash
cargo run --release
```

> **Note:** Since this app uses raw terminal mode and UDP broadcasting, you may need to allow it through your firewall on the first run.

## Usage

Once the application starts, you will see the Sandesh prompt.

### Commands

| Command | Description |
| --- | --- |
| `find` | Opens a live monitor to scan for active peers on the LAN. |
| `find-quick` | Prints a snapshot list of currently known peers without leaving the prompt. |
| `connect <IP>` | Initiates a secure chat session with a specific IP (Port defaults to 3001). |
| `cls` / `clear` | Clears the terminal screen and redraws the banner. |
| `exit` | Closes the application and stops background threads. |

### Navigation

* **Up/Down Arrows:** Cycle through command history.
* **PageUp/PageDown:** Scroll through chat history during an active session.
* **Esc:** Disconnect from a chat or exit the `find` monitor.

## Architecture

The codebase is modularized into four key components:

### 1. `main.rs` (The Controller)

Handles the main event loop, TUI rendering for the menu, input processing, and command history. It acts as the bridge between user input and the network state.

### 2. `network.rs` (The Nervous System)

Manages background threads:

* **Listener Thread:** Listens for UDP broadcast packets (`HELLO_P2P`) to update the peer list.
* **Broadcaster Thread:** Sends a heartbeat every 5 seconds to announce presence to the LAN.
* **Cleanup Thread:** Removes peers that haven't been seen in 15 seconds.
* **TCP Listener:** Listens for incoming chat requests.

### 3. `crypto.rs` (The Shield)

Implements the security layer:

* **Handshake:** Uses `x25519_dalek` to generate ephemeral key pairs. Performs a Diffie-Hellman key exchange to derive a shared secret.
* **Encryption:** Uses `ChaCha20Poly1305` (AEAD) to encrypt messages. A random unique Nonce is generated for every message sent to prevent replay attacks.

### 4. `chat.rs` (The View)

Manages the active chat session state. It handles the specific UI logic for the split-screen chat view (messages on top, input on bottom) and handles the blocking/non-blocking read logic for TCP streams.

## Dependencies

Add the following to your `Cargo.toml` to build the project:

```toml
[dependencies]
crossterm = "0.27"
colored = "2.0"
rand = "0.8"
chacha20poly1305 = "0.10"
x25519-dalek = "2.0"
byteorder = "1.5"
```

## ⚠️ Security Disclaimer

This application is intended for educational purposes. While it uses industry-standard algorithms (ChaCha20, X25519), the protocol implementation has not been professionally audited. Use with caution for sensitive communications.

## 🤝 Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

---

**Built with ❤️ in Rust.**

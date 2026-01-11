use chacha20poly1305::{
    aead::{Aead},
    ChaCha20Poly1305, Nonce,
};
use rand::{rngs::OsRng, RngCore};
use x25519_dalek::{EphemeralSecret, PublicKey};
use std::io::{self, Read, Write};
use std::net::TcpStream;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

pub fn generate_keypair() -> (EphemeralSecret, PublicKey) {
    let secret = EphemeralSecret::random_from_rng(OsRng);
    let public = PublicKey::from(&secret);
    (secret, public)
}

pub fn perform_handshake(mut stream: &TcpStream) -> io::Result<[u8; 32]> {
    let (our_secret, our_public) = generate_keypair();
    let our_pub_bytes = our_public.as_bytes();

    stream.write_all(our_pub_bytes)?;

    let mut peer_pub_bytes = [0u8; 32];
    stream.read_exact(&mut peer_pub_bytes)?;
    let peer_public = PublicKey::from(peer_pub_bytes);

    let shared_secret = our_secret.diffie_hellman(&peer_public);
    Ok(*shared_secret.as_bytes())
}

pub fn encrypt_and_send(stream: &mut TcpStream, cipher: &ChaCha20Poly1305, msg: &str) -> io::Result<()> {
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, msg.as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "Encryption failed"))?;

    let total_len = 12 + ciphertext.len(); 
    
    stream.write_u32::<BigEndian>(total_len as u32)?; 
    stream.write_all(&nonce_bytes)?; 
    stream.write_all(&ciphertext)?; 
    
    Ok(())
}

pub fn receive_and_decrypt(stream: &mut TcpStream, cipher: &ChaCha20Poly1305) -> io::Result<String> {
    // 1. PEEK
    let mut len_buf = [0u8; 4];
    match stream.peek(&mut len_buf) {
        Ok(4) => { /* Header ready */ },
        
        // FIX: Explicitly check for 0. This means the connection is closed.
        Ok(0) => return Err(io::Error::new(io::ErrorKind::ConnectionAborted, "Peer disconnected")),
        
        // Less than 4 bytes means data is trickling in, but not ready yet.
        Ok(_) => return Err(io::Error::from(io::ErrorKind::WouldBlock)),
        
        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => return Err(io::Error::from(io::ErrorKind::WouldBlock)),
        Err(e) => return Err(e),
    }

    // 2. READ LENGTH
    let len = stream.read_u32::<BigEndian>()?;
    if len < 12 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Message too short"));
    }

    // 3. TOGGLE BLOCKING
    stream.set_nonblocking(false)?;

    let mut buffer = vec![0u8; len as usize];
    let read_result = stream.read_exact(&mut buffer);

    // 4. RESTORE NON-BLOCKING
    stream.set_nonblocking(true)?;

    match read_result {
        Ok(_) => {},
        // If the peer disconnects *during* the body transmission
        Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => {
             return Err(io::Error::new(io::ErrorKind::ConnectionAborted, "Peer disconnected"));
        }
        Err(e) => return Err(e),
    }

    // 5. DECRYPT
    let (nonce_bytes, ciphertext_bytes) = buffer.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext_bytes = cipher.decrypt(nonce, ciphertext_bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Decryption failed"))?;

    let plaintext = String::from_utf8(plaintext_bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF8"))?;

    Ok(plaintext)
}

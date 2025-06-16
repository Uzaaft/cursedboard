use std::io::Read;
use std::net::{TcpListener, TcpStream};

fn handle_client(mut stream: TcpStream, address: std::net::SocketAddr) {
    println!("Connection received! {} is sending data.", address);

    // Program flow:
    // First 8 bytes contains length of the message.
    // The rest of the bytes contains the message, with the aforementioned length
    let mut message_length = [0u8; 8];
    match stream.read_exact(&mut message_length) {
        Ok(_) => {
            let length = u64::from_le_bytes(message_length);
            println!("{} says message is: {} bytes long", address, length);
        }
        Err(e) => {
            eprintln!("Failed to read message length: {}", e);
        }
    }
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:34254")?;
    let addr = listener.local_addr()?;
    
    println!("Listening on {}, access this port to end the program", addr.port());
    
    // Accept a single connection (matching the Zig behavior)
    match listener.accept() {
        Ok((stream, address)) => {
            handle_client(stream, address);
        }
        Err(e) => {
            eprintln!("Failed to accept connection: {}", e);
        }
    }
    
    Ok(())
}
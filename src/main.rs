
mod auth;
mod chat;

use chat::SecureConnection;
use snow::{Builder, params::NoiseParams};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const ADDRESS: &str = "0.0.0.0:1342";
const KEY_FILE: &str = "server.key";
const PARAMS: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";
const RATE_LIMIT: Duration = Duration::from_millis(500);

type Clients = Arc<Mutex<HashMap<String, Arc<Mutex<SecureConnection>>>>>;

fn read_frame(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
    let mut size = [0u8; 4];
    stream.read_exact(&mut size).map_err(|e| e.to_string())?;

    let len = u32::from_be_bytes(size) as usize;
    if len == 0 || len > 65536 {
        return Err("Invalid frame".into());
    }

    let mut data = vec![0; len];
    stream.read_exact(&mut data).map_err(|e| e.to_string())?;
    Ok(data)
}

fn write_frame(stream: &mut TcpStream, data: &[u8]) -> Result<(), String> {
    let size = (data.len() as u32).to_be_bytes();
    stream.write_all(&size).map_err(|e| e.to_string())?;
    stream.write_all(data).map_err(|e| e.to_string())
}

fn server_key() -> Vec<u8> {
    if let Ok(key) = fs::read(KEY_FILE) {
        return key;
    }

    let params: NoiseParams = PARAMS.parse().unwrap();
    let keypair = Builder::new(params).generate_keypair().unwrap();

    fs::write(KEY_FILE, &keypair.private).expect("Cannot save server key");

    println!(
        "SERVER PUBLIC KEY: {}",
        keypair.public.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    );

    keypair.private
}

fn handshake(stream: &mut TcpStream, private: &[u8])
    -> Result<SecureConnection, String>
{
    let params: NoiseParams = PARAMS.parse().map_err(|e| format!("{e}"))?;

    let builder = Builder::new(params)
        .local_private_key(private);

    let mut noise = builder.build_responder()
        .map_err(|e| e.to_string())?;

    let mut buffer = vec![0u8; 65536];

    // Message 1
    let msg1 = read_frame(stream)?;
    noise.read_message(&msg1, &mut buffer)
        .map_err(|e| e.to_string())?;

    // Message 2
    let len = noise.write_message(&[], &mut buffer)
        .map_err(|e| e.to_string())?;

    write_frame(stream, &buffer[..len])?;

    // Message 3
    let msg3 = read_frame(stream)?;
    noise.read_message(&msg3, &mut buffer)
        .map_err(|e| e.to_string())?;

    // XX handshake completed.
    let transport = noise.into_transport_mode()
        .map_err(|e| e.to_string())?;

    Ok(SecureConnection::new(stream.try_clone().map_err(|e| e.to_string())?, transport))
}

fn broadcast(clients: &Clients, sender: &str, message: &str) {
    let clients = clients.lock().unwrap();

    for (name, client) in clients.iter() {
        if name != sender {
            let _ = client.lock().unwrap()
                .send(&format!("{}> {}", sender, message));
        }
    }
}

fn handle_client(
    mut stream: TcpStream,
    clients: Clients,
    private: Arc<Vec<u8>>,
) {
    let mut secure = match handshake(&mut stream, &private) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Handshake failed: {e}");
            return;
        }
    };

    if secure.send("Connected securely. 1=Register, 2=Login").is_err() {
        return;
    }

    let choice = match secure.receive() {
        Ok(v) => v,
        Err(_) => return,
    };

    let username = match secure.receive() {
        Ok(v) => v,
        Err(_) => return,
    };

    if !auth::validate_username(&username) {
        let _ = secure.send("Invalid username.");
        return;
    }

    let register = choice == "1";

    if !register && choice != "2" {
        let _ = secure.send("Invalid choice.");
        return;
    }

    let mut authenticated = false;

    let attempts = if register { 1 } else { 3 };

    for attempt in 0..attempts {
        if secure.send("Password:").is_err() {
            return;
        }

        let password = match secure.receive() {
            Ok(v) => v,
            Err(_) => return,
        };

        if auth::authenticate(&username, &password, register).is_ok() {
            authenticated = true;
            break;
        }

        let remaining = attempts - attempt - 1;

        if secure.send(&format!("Wrong password. Remaining: {}", remaining)).is_err() {
            return;
        }
    }

    if !authenticated {
        let _ = secure.send("Authentication failed. Disconnecting.");
        return;
    }

    let connection = Arc::new(Mutex::new(secure));

    {
        let mut online = clients.lock().unwrap();

        if online.contains_key(&username) {
            let _ = connection.lock().unwrap().send("Already online.");
            return;
        }

        online.insert(username.clone(), connection.clone());
    }

    let _ = connection.lock().unwrap().send("Login successful.");
    broadcast(&clients, &username, "joined the chat");

    let mut last_sent = Instant::now();

    loop {
        let message = match connection.lock().unwrap().receive() {
            Ok(v) => v,
            Err(_) => break,
        };

        let message = message.trim();

        if message.is_empty() {
            continue;
        }

        if message == "/exit" {
            let _ = connection.lock().unwrap().send("Goodbye.");
            break;
        }

        if last_sent.elapsed() < RATE_LIMIT {
            let _ = connection.lock().unwrap()
                .send("You are sending messages too fast.");
            continue;
        }

        last_sent = Instant::now();

        if message == "/online" {
            let names = clients.lock().unwrap()
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");

            let _ = connection.lock().unwrap()
                .send(&format!("Online: {}", names));
            continue;
        }

        if let Some(rest) = message.strip_prefix("/msg ") {
            if let Some((target, text)) = rest.split_once(' ') {
                let clients = clients.lock().unwrap();

                if let Some(client) = clients.get(target) {
                    let _ = client.lock().unwrap()
                        .send(&format!("(private) {}: {}", username, text));
                } else {
                    let _ = connection.lock().unwrap()
                        .send("User not online.");
                }
            }

            continue;
        }

        let _ = connection.lock().unwrap()
            .send(&format!("you> {}", message));

        broadcast(&clients, &username, message);
    }

    clients.lock().unwrap().remove(&username);
    broadcast(&clients, &username, "left the chat");

    println!("{username} disconnected");
}

fn main() {
    let private = Arc::new(server_key());

    let listener = TcpListener::bind(ADDRESS)
        .expect("Server could not start");

    let clients: Clients = Arc::new(Mutex::new(HashMap::new()));

    println!("Secure chat running on {ADDRESS}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let clients = Arc::clone(&clients);
                let private = Arc::clone(&private);

                thread::spawn(move || {
                    handle_client(stream, clients, private);
                });
            }
            Err(e) => eprintln!("Connection error: {e}"),
        }
    }
}

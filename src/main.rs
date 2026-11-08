use bcrypt::{hash, verify, DEFAULT_COST};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const USERS_FILE: &str = "users.json";
const RATE_LIMIT: Duration = Duration::from_millis(500);

#[derive(Serialize, Deserialize)]
struct User {
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize, Default)]
struct Users {
    users: Vec<User>,
}

struct Client {
    conn: TcpStream,
    last_sent: Instant,
}

type Clients = Arc<Mutex<HashMap<String, Client>>>;

fn load_users() -> Users {
    match fs::read_to_string(USERS_FILE) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Users::default(),
    }
}

fn save_users(users: &Users) {
    let json = serde_json::to_string_pretty(users)
        .expect("JSON serialization failed");

    fs::write(USERS_FILE, json).expect("Failed to write users.json");
}

fn user_exists(users: &Users, username: &str) -> bool {
    users.users.iter().any(|user| user.username == username)
}

fn validate_username(username: &str) -> bool {
    username.len() >= 3
        && username.len() <= 20
        && username.chars().all(|c| {
            c.is_ascii_alphanumeric() || c == '_'
        })
}

fn send(stream: &mut TcpStream, message: &str) {
    let _ = stream.write_all(message.as_bytes());
}

fn broadcast(clients: &Clients, sender: &str, message: &str) {
    let mut clients = clients.lock().unwrap();

    for (username, client) in clients.iter_mut() {
    	if username != sender {
    		let _ = client.conn.write_all(
    			format!("{}> {}\n", sender, message).as_bytes()
            );
        }
}
}

fn private_message(clients: &Clients, sender: &str, target: &str,    message: &str,) {
    let clients = clients.lock().unwrap();

    if let Some(client) = clients.get(target) {
        if let Ok(mut conn) = client.conn.try_clone() {
            let _ = conn.write_all(
                format!("(private) {}> {}\n", sender, message).as_bytes()
            );
        }
    }
}

fn remove_client(clients: &Clients, username: &str) {
    let mut clients = clients.lock().unwrap();
    clients.remove(username);
}

fn handle_client(mut conn: TcpStream, clients: Clients) {
    let reader_conn = match conn.try_clone() {
        Ok(c) => c,
        Err(_) => return,
    };

    let mut reader = BufReader::new(reader_conn);

    send(&mut conn, "Connected to server\n");
    send(&mut conn, "1) Register\n2) Login\n> ");

    let mut choice = String::new();

    if reader.read_line(&mut choice).is_err() {
        return;
    }

    let choice = choice.trim().to_string();

    send(&mut conn, "Username: ");

    let mut username = String::new();

    if reader.read_line(&mut username).is_err() {
        return;
    }

    let username = username.trim().to_string();

    if !validate_username(&username) {
        send(&mut conn, "Invalid username.\n");
        return;
    }

    send(&mut conn, "Password: ");

    let mut password = String::new();

    if reader.read_line(&mut password).is_err() {
        return;
    }

    let password = password.trim().to_string();

    let mut users = load_users();

    match choice.as_str() {
        "1" => {
            if user_exists(&users, &username) {
                send(&mut conn, "User already exists.\n");
                return;
            }

            let hashed = match hash(&password, DEFAULT_COST) {
                Ok(h) => h,
                Err(_) => {
                    send(&mut conn, "Server error.\n");
                    return;
                }
            };

            users.users.push(User {
                username: username.clone(),
                password: hashed,
            });

            if let Err(_) = fs::write(
                USERS_FILE,
                serde_json::to_string_pretty(&users).unwrap(),
            ) {
                send(&mut conn, "Failed to save user.\n");
                return;
            }

            send(&mut conn, "Registered successfully.\n");
        }
        "2" => {
            let stored_password = users
                .users
                .iter()
                .find(|user| user.username == username)
                .map(|user| user.password.clone());

            let stored_password = match stored_password {
                Some(password) => password,
                None => {
                    send(&mut conn, "User not found.\n");
                    return;
                }
            };

            if !verify(&password, &stored_password).unwrap_or(false) {
                send(&mut conn, "Wrong password.\n");
                return;
            }

            send(&mut conn, "Login successful.\n");
        }
        _ => {
            send(&mut conn, "Invalid choice.\n");
            return;
        }
    }

    let client_conn = match conn.try_clone() {
        Ok(c) => c,
        Err(_) => return,
    };

    {
        let mut online = clients.lock().unwrap();

        if online.contains_key(&username) {
            send(&mut conn, "User already online.\n");
            return;
        }

        online.insert(
            username.clone(),
            Client {
                conn: client_conn,
                last_sent: Instant::now(),
            },
        );
    }

    broadcast(&clients, &username, "joined the chat");

    send(&mut conn, &format!("Welcome {}\n", username));

    loop {
        let mut msg = String::new();

        match reader.read_line(&mut msg) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }

        let msg = msg.trim().to_string();

        if msg.is_empty() {
            continue;
        }

        if msg == "/exit" {
            send(&mut conn, "Goodbye.\n");
            break;
        }

        {
            let mut online = clients.lock().unwrap();

            if let Some(client) = online.get_mut(&username) {
                if client.last_sent.elapsed() < RATE_LIMIT {
                    send(&mut conn, "You are sending messages too fast.\n");
                    continue;
                }

                client.last_sent = Instant::now();
            } else {
                break;
            }
        }

        if msg == "/online" {
            let online = clients.lock().unwrap();

            send(&mut conn, "Online users:\n");

            for user in online.keys() {
                send(&mut conn, &format!("- {}\n", user));
            }

            continue;
        }

        if let Some(rest) = msg.strip_prefix("/msg ") {
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();

            if parts.len() == 2 {
                private_message(
                    &clients,
                    &username,
                    parts[0],
                    parts[1],
                );
            }

            continue;
        }

        send(&mut conn, &format!("you> {}\n", msg));
        broadcast(&clients, &username, &msg);
    }

    remove_client(&clients, &username);
    broadcast(&clients, &username, "left the chat");

    println!("{} disconnected", username);
}

fn main() {
    if !std::path::Path::new(USERS_FILE).exists() {
        save_users(&Users::default());
    }

    let listener = TcpListener::bind("0.0.0.0:1342")
        .expect("Could not start server");

    let clients: Clients = Arc::new(Mutex::new(HashMap::new()));

    println!("Server running on 0.0.0.0:1342");

    for stream in listener.incoming() {
        match stream {
            Ok(conn) => {
                let clients = Arc::clone(&clients);

                thread::spawn(move || {
                    handle_client(conn, clients);
                });
            }

            Err(e) => {
                eprintln!("Connection error: {}", e);
            }
        }
    }
}

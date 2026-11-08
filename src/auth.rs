
use bcrypt::{hash, verify, DEFAULT_COST};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Mutex;

const USERS_FILE: &str = "users.json";
const MAX_ATTEMPTS: u8 = 3;

#[derive(Serialize, Deserialize, Clone)]
pub struct User {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Users {
    pub users: Vec<User>,
}

static DB_LOCK: Mutex<()> = Mutex::new(());

fn load_users() -> Users {
    let data = fs::read_to_string(USERS_FILE).unwrap_or_default();
    serde_json::from_str(&data).unwrap_or_default()
}

fn save_users(users: &Users) -> Result<(), String> {
    let json = serde_json::to_string_pretty(users)
        .map_err(|e| e.to_string())?;

    fs::write(USERS_FILE, json)
        .map_err(|e| e.to_string())
}

pub fn validate_username(username: &str) -> bool {
    (3..=20).contains(&username.len())
        && username.chars().all(|c|
            c.is_ascii_alphanumeric() || c == '_'
        )
}

pub fn authenticate(
    username: &str,
    password: &str,
    register: bool,
) -> Result<(), String> {
    let _guard = DB_LOCK.lock().map_err(|e| e.to_string())?;
    let mut users = load_users();

    if register {
        if !validate_username(username) {
            return Err("Invalid username".into());
        }

        if password.len() < 8 || password.len() > 72 {
            return Err("Password must be 8-72 bytes".into());
        }

        if users.users.iter().any(|u| u.username == username) {
            return Err("User already exists".into());
        }

        let hashed = hash(password, DEFAULT_COST)
            .map_err(|e| e.to_string())?;

        users.users.push(User {
            username: username.to_string(),
            password: hashed,
        });

        save_users(&users)?;
        return Ok(());
    }

    let stored = users.users.iter()
        .find(|u| u.username == username)
        .map(|u| u.password.clone());

    let stored = stored.ok_or("Invalid username or password")?;

    if verify(password, &stored).unwrap_or(false) {
        Ok(())
    } else {
        Err("Invalid username or password".into())
    }
}

pub fn login(
    username: &str,
    register: bool,
    mut read_password: impl FnMut() -> String,
) -> Result<(), String> {
    if register {
        let password = read_password();
        return authenticate(username, &password, true);
    }

    for attempt in 1..=MAX_ATTEMPTS {
        let password = read_password();

        if authenticate(username, &password, false).is_ok() {
            return Ok(());
        }

        if attempt == MAX_ATTEMPTS {
            return Err("Three incorrect attempts. Disconnected.".into());
        }
    }

    Err("Authentication failed".into())
}

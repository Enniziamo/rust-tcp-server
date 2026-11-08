# 🦀 Rust TCP Chat Server

A lightweight, multi-client TCP chat server written in Rust. This project provides a simple real-time messaging system with user authentication, private messaging, and online user management.

Built using Rust's standard networking and threading capabilities, with JSON-based user storage and bcrypt password hashing.

# ✨ Features
TCP Networking: Communication between multiple clients using TCP connections.
Multi-client Support: Handles multiple clients concurrently using threads.
User Registration: Create accounts with unique usernames.
User Authentication: Secure password verification using bcrypt.
Password Hashing: Passwords are stored as bcrypt hashes instead of plaintext.
Global Chat: Send messages to all connected users.
Private Messaging: Send direct messages to specific users.
Online User List: Display currently connected users.
Rate Limiting: Prevents users from sending messages too frequently.
Connection Management: Automatically handles client disconnections.
JSON Storage: Stores user information locally in users.json.
🛠️ Technologies
Technology	Purpose
Rust	Core programming language
TCP	Client-server communication
std::net	TCP networking
std::thread	Concurrent client handling
Arc<Mutex<T>>	Shared client state management
Serde	Serialization and deserialization
Serde JSON	JSON-based user storage
bcrypt	Password hashing and verification
📦 Requirements
Rust (2024 Edition support)
Cargo

Install Rust using rustup.

# 🚀 Installation

Clone the repository:

bash
git clone https://github.com/Enniziamo/rust-tcp-server.git


Navigate to the project directory:

bash
cd rust-tcp-server


Build the project:

bash
cargo build --release

# ▶️ Usage

Start the server:

bash
cargo run --release


The server listens on:

text
0.0.0.0:1342


Connect to the server using a TCP client:

bash
nc 127.0.0.1 1342


If the server is running on another machine, replace 127.0.0.1 with its IP address.

# 🔐 Authentication

When connecting to the server, you will be presented with two options:

text
Connected to server
1) Register
2) Login
>

Register

Choose 1 to create a new account.

You will be asked to provide:

Username (3–20 characters)
Password

Usernames can contain ASCII letters, numbers, and underscores.

Login

Choose 2 to log in with an existing account.

Passwords are verified against their stored bcrypt hashes.

💬 Chat Commands

Once authenticated, you can use the following commands:

Command	Description
/online	Display all currently connected users
/msg <username> <message>	Send a private message
/exit	Disconnect from the server
Global Messaging

Simply type a message and press Enter to broadcast it to other connected users.

Private Messaging

Send a private message using:

text
/msg alice Hello! How are you?


Only the specified recipient receives the private message.

Online Users

To see who is currently connected:

text
/online

Disconnect

To leave the chat:

text
/exit

📁 Project Structure
text
rust-tcp-server/
│
├── src/
│   └── main.rs
│
├── Cargo.toml
├── Cargo.lock
├── .gitignore
└── README.md


The users.json file is generated automatically when the server starts for the first time. It stores registered users and their password hashes.

⚙️ Configuration

The server currently uses the following default settings:

Setting	Value
Bind Address	0.0.0.0
Port	1342
Username Length	3–20 characters
Message Rate Limit	500 milliseconds
Password Hashing	bcrypt
🧠 Architecture

The server uses a thread-per-client architecture.

TcpListener accepts incoming TCP connections.
Each client is handled in a separate thread.
Arc<Mutex<HashMap<String, Client>>> manages shared client connections.
BufReader handles incoming messages.
TcpStream handles communication.
Serde manages JSON serialization.
bcrypt handles password hashing and verification.
⚠️ Notes
The server uses an unencrypted TCP connection. Do not expose it to untrusted networks without adding transport encryption.
User data is stored locally in users.json.
The current implementation is intended as a learning and experimental project rather than a production-ready messaging service.
📌 Future Improvements

Potential improvements include:

TLS encryption
Persistent database integration (SQLite or PostgreSQL)
Improved authentication and session management
Chat rooms and channels
Message history
Better error handling
Asynchronous networking with Tokio
Unit and integration tests
📄 License

No license has currently been specified. Please check the repository for licensing information before redistributing or modifying the project.

Built with ❤️ and Rust.

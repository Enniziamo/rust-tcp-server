
use snow::TransportState;
use std::io::{Read, Write};
use std::net::TcpStream;

pub struct SecureConnection {
    stream: TcpStream,
    noise: TransportState,
}

impl SecureConnection {
    pub fn new(stream: TcpStream, noise: TransportState) -> Self {
        Self { stream, noise }
    }

    pub fn send(&mut self, message: &str) -> Result<(), String> {
        let mut encrypted = vec![0u8; message.len() + 64];

        let len = self.noise.write_message(
            message.as_bytes(),
            &mut encrypted,
        ).map_err(|e| e.to_string())?;

        encrypted.truncate(len);

        let size = (encrypted.len() as u32).to_be_bytes();

        self.stream.write_all(&size)
            .map_err(|e| e.to_string())?;

        self.stream.write_all(&encrypted)
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn receive(&mut self) -> Result<String, String> {
        let mut size = [0u8; 4];

        self.stream.read_exact(&mut size)
            .map_err(|e| e.to_string())?;

        let len = u32::from_be_bytes(size) as usize;

        if len == 0 || len > 65536 {
            return Err("Invalid frame size".into());
        }

        let mut encrypted = vec![0u8; len];

        self.stream.read_exact(&mut encrypted)
            .map_err(|e| e.to_string())?;

        let mut decrypted = vec![0u8; len];

        let actual = self.noise.read_message(
            &encrypted,
            &mut decrypted,
        ).map_err(|e| e.to_string())?;

        decrypted.truncate(actual);

        String::from_utf8(decrypted)
            .map_err(|e| e.to_string())
    }
}

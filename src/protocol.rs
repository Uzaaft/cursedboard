use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("invalid message length")]
    InvalidLength,
    #[error("invalid message format: {0}")]
    InvalidFormat(#[from] toml::de::Error),
    #[error("authentication failed")]
    AuthFailed,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Hello { id: Uuid, name: String },
    Auth { challenge: [u8; 32], response: [u8; 32] },
    Clipboard { content: String, timestamp: u64 },
    Ping,
    Pong,
}

impl Message {
    pub fn encode(&self) -> Vec<u8> {
        let payload = toml::to_string(self).expect("message serialization should not fail");
        let len = payload.len() as u32;
        let mut buf = Vec::with_capacity(4 + payload.len());
        buf.extend_from_slice(&len.to_be_bytes());
        buf.extend_from_slice(payload.as_bytes());
        buf
    }

    pub fn decode(data: &[u8]) -> Result<Self, ProtocolError> {
        if data.len() < 4 {
            return Err(ProtocolError::InvalidLength);
        }
        let len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        if data.len() < 4 + len {
            return Err(ProtocolError::InvalidLength);
        }
        let payload = std::str::from_utf8(&data[4..4 + len])
            .map_err(|_| ProtocolError::InvalidLength)?;
        Ok(toml::from_str(payload)?)
    }
}

pub fn compute_auth_response(psk: &str, challenge: &[u8; 32]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(psk.as_bytes())
        .expect("HMAC accepts any key length");
    mac.update(challenge);
    let result = mac.finalize();
    let mut response = [0u8; 32];
    response.copy_from_slice(&result.into_bytes());
    response
}

pub fn verify_auth_response(psk: &str, challenge: &[u8; 32], response: &[u8; 32]) -> bool {
    let expected = compute_auth_response(psk, challenge);
    constant_time_eq(&expected, response)
}

fn constant_time_eq(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

pub fn generate_challenge() -> [u8; 32] {
    let mut challenge = [0u8; 32];
    getrandom(&mut challenge);
    challenge
}

fn getrandom(buf: &mut [u8]) {
    use std::fs::File;
    use std::io::Read;
    
    #[cfg(unix)]
    {
        let mut f = File::open("/dev/urandom").expect("failed to open /dev/urandom");
        f.read_exact(buf).expect("failed to read random bytes");
    }
    
    #[cfg(windows)]
    {
        use std::ptr;
        extern "system" {
            fn BCryptGenRandom(
                hAlgorithm: *mut std::ffi::c_void,
                pbBuffer: *mut u8,
                cbBuffer: u32,
                dwFlags: u32,
            ) -> i32;
        }
        unsafe {
            BCryptGenRandom(ptr::null_mut(), buf.as_mut_ptr(), buf.len() as u32, 2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_roundtrip() {
        let msg = Message::Clipboard {
            content: "hello".into(),
            timestamp: 12345,
        };
        let encoded = msg.encode();
        let decoded = Message::decode(&encoded).unwrap();
        match decoded {
            Message::Clipboard { content, timestamp } => {
                assert_eq!(content, "hello");
                assert_eq!(timestamp, 12345);
            }
            _ => panic!("wrong message type"),
        }
    }

    #[test]
    fn test_auth_verify() {
        let psk = "secret";
        let challenge = generate_challenge();
        let response = compute_auth_response(psk, &challenge);
        assert!(verify_auth_response(psk, &challenge, &response));
        assert!(!verify_auth_response("wrong", &challenge, &response));
    }
}

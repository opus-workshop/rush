//! Rush daemon protocol implementation
//!
//! Message format: Length-prefixed binary messages with JSON payloads
//!
//! ```text
//! ┌────────────┬──────────────┬─────────────────────┐
//! │   Length   │  Message ID  │   Payload (JSON)    │
//! │  (4 bytes) │  (4 bytes)   │  (variable length)  │
//! └────────────┴──────────────┴─────────────────────┘
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, RawFd};

/// Maximum message size (10MB to prevent memory exhaustion)
const MAX_MESSAGE_SIZE: u32 = 10 * 1024 * 1024;

/// Message ID counter type (unique per message for request/response correlation)
pub type MessageId = u32;

/// Message envelope containing all possible message types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    /// Client initiates a new session
    SessionInit(SessionInit),
    /// Daemon acknowledges session creation
    SessionInitAck(SessionInitAck),
    /// Client requests command execution
    Execute(Execute),
    /// Daemon returns execution result
    ExecutionResult(ExecutionResult),
    /// Client sends signal to running command
    Signal(Signal),
    /// Client requests daemon shutdown
    Shutdown(Shutdown),
}

/// Session initialization request (Client → Daemon)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionInit {
    /// Working directory for the session
    pub working_dir: String,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Command-line arguments
    pub args: Vec<String>,
    /// How to handle stdin: "inherit", "pipe", or "null"
    pub stdin_mode: String,
}

/// Session initialization acknowledgment (Daemon → Client)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionInitAck {
    /// Unique session ID
    pub session_id: u64,
    /// Worker process PID
    pub worker_pid: i32,
}

/// Command execution request (Client → Daemon)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Execute {
    /// Session ID to execute in
    pub session_id: u64,
    /// Command string to execute
    pub command: String,
}

/// Execution result response (Daemon → Client)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Exit code of the command
    pub exit_code: i32,
    /// Number of bytes written to stdout
    pub stdout_len: u64,
    /// Number of bytes written to stderr
    pub stderr_len: u64,
    /// Standard output content
    pub stdout: String,
    /// Standard error content
    pub stderr: String,
}

/// Signal delivery request (Client → Daemon)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Signal {
    /// Session ID to signal
    pub session_id: u64,
    /// Signal number (e.g., SIGINT=2, SIGTERM=15)
    pub signal: i32,
}

/// Daemon shutdown request (Client → Daemon)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shutdown {
    /// Whether to force shutdown (kill active sessions)
    pub force: bool,
}

/// Encode a message into the wire format
///
/// Format: [4-byte length][4-byte message ID][JSON payload]
pub fn encode_message(message: &Message, message_id: MessageId) -> io::Result<Vec<u8>> {
    // Serialize the message to JSON
    let payload = serde_json::to_vec(message)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    // Check size limit
    let payload_len = payload.len() as u32;
    if payload_len > MAX_MESSAGE_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Message too large: {} bytes", payload_len),
        ));
    }

    // Build the frame: length (4) + message_id (4) + payload
    let total_len = 8 + payload_len;
    let mut buffer = Vec::with_capacity(total_len as usize);

    // Write length prefix (includes message_id + payload)
    buffer.extend_from_slice(&(payload_len + 4).to_le_bytes());
    // Write message ID
    buffer.extend_from_slice(&message_id.to_le_bytes());
    // Write payload
    buffer.extend_from_slice(&payload);

    Ok(buffer)
}

/// Decode a message from the wire format
///
/// Returns (message, message_id)
pub fn decode_message<R: Read>(reader: &mut R) -> io::Result<(Message, MessageId)> {
    // Read length prefix (4 bytes)
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes)?;
    let payload_len = u32::from_le_bytes(len_bytes);

    // Validate length
    if payload_len < 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Message length too small",
        ));
    }
    if payload_len > MAX_MESSAGE_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Message too large: {} bytes", payload_len),
        ));
    }

    // Read message ID (4 bytes)
    let mut id_bytes = [0u8; 4];
    reader.read_exact(&mut id_bytes)?;
    let message_id = u32::from_le_bytes(id_bytes);

    // Read JSON payload
    let json_len = (payload_len - 4) as usize;
    let mut payload = vec![0u8; json_len];
    reader.read_exact(&mut payload)?;

    // Deserialize message
    let message: Message = serde_json::from_slice(&payload)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    Ok((message, message_id))
}

/// Write a message to a stream
pub fn write_message<W: Write>(
    writer: &mut W,
    message: &Message,
    message_id: MessageId,
) -> io::Result<()> {
    let bytes = encode_message(message, message_id)?;
    writer.write_all(&bytes)?;
    writer.flush()?;
    Ok(())
}

/// Read a message from a stream
pub fn read_message<R: Read>(reader: &mut R) -> io::Result<(Message, MessageId)> {
    decode_message(reader)
}

// Unix FD passing helpers using SCM_RIGHTS

/// Send file descriptors over a Unix socket using SCM_RIGHTS
///
/// This allows zero-copy I/O by passing stdin/stdout/stderr FDs directly
pub fn send_fds<S: AsRawFd>(socket: &S, fds: &[RawFd]) -> io::Result<()> {
    use nix::sys::socket::{sendmsg, ControlMessage, MsgFlags};

    // We need to send at least 1 byte of data along with the FDs
    let dummy_data = [0u8; 1];
    let iov = [std::io::IoSlice::new(&dummy_data)];

    // Create SCM_RIGHTS control message
    let fds_msg = ControlMessage::ScmRights(fds);

    // Send the message
    sendmsg::<()>(
        socket.as_raw_fd(),
        &iov,
        &[fds_msg],
        MsgFlags::empty(),
        None,
    )
    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    Ok(())
}

/// Receive file descriptors from a Unix socket using SCM_RIGHTS
///
/// Returns the received file descriptors
pub fn recv_fds<S: AsRawFd>(socket: &S, max_fds: usize) -> io::Result<Vec<RawFd>> {
    use nix::sys::socket::{recvmsg, ControlMessageOwned, MsgFlags};

    // Buffer to receive the dummy data
    let mut dummy_data = [0u8; 1];
    let mut iov = [std::io::IoSliceMut::new(&mut dummy_data)];

    // Buffer for control messages (each FD is 4 bytes)
    let mut cmsg_buffer = nix::cmsg_space!([RawFd; 16]);

    // Receive the message
    let msg = recvmsg::<()>(
        socket.as_raw_fd(),
        &mut iov,
        Some(&mut cmsg_buffer),
        MsgFlags::empty(),
    )
    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // Extract file descriptors from control messages
    let mut fds = Vec::new();

    // Note: In nix 0.29, cmsgs() returns a Result<CmsgIterator>
    // The iterator yields ControlMessageOwned items
    match msg.cmsgs() {
        Ok(cmsgs_iter) => {
            for cmsg in cmsgs_iter {
                if let ControlMessageOwned::ScmRights(received_fds) = cmsg {
                    fds.extend(received_fds.iter().take(max_fds - fds.len()));
                    if fds.len() >= max_fds {
                        break;
                    }
                }
            }
        }
        Err(e) => {
            return Err(io::Error::new(io::ErrorKind::Other, e));
        }
    }

    Ok(fds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_session_init() {
        let mut env = HashMap::new();
        env.insert("PATH".to_string(), "/usr/bin".to_string());
        env.insert("HOME".to_string(), "/home/test".to_string());

        let message = Message::SessionInit(SessionInit {
            working_dir: "/tmp".to_string(),
            env,
            args: vec!["-c".to_string(), "echo test".to_string()],
            stdin_mode: "inherit".to_string(),
        });

        let message_id = 42;
        let encoded = encode_message(&message, message_id).unwrap();

        // Decode from bytes
        let mut cursor = std::io::Cursor::new(encoded);
        let (decoded, decoded_id) = decode_message(&mut cursor).unwrap();

        assert_eq!(message, decoded);
        assert_eq!(message_id, decoded_id);
    }

    #[test]
    fn test_encode_decode_session_init_ack() {
        let message = Message::SessionInitAck(SessionInitAck {
            session_id: 12345,
            worker_pid: 98765,
        });

        let message_id = 43;
        let encoded = encode_message(&message, message_id).unwrap();

        let mut cursor = std::io::Cursor::new(encoded);
        let (decoded, decoded_id) = decode_message(&mut cursor).unwrap();

        assert_eq!(message, decoded);
        assert_eq!(message_id, decoded_id);
    }

    #[test]
    fn test_encode_decode_execute() {
        let message = Message::Execute(Execute {
            session_id: 12345,
            command: "ls -la".to_string(),
        });

        let message_id = 44;
        let encoded = encode_message(&message, message_id).unwrap();

        let mut cursor = std::io::Cursor::new(encoded);
        let (decoded, decoded_id) = decode_message(&mut cursor).unwrap();

        assert_eq!(message, decoded);
        assert_eq!(message_id, decoded_id);
    }

    #[test]
    fn test_encode_decode_execution_result() {
        let message = Message::ExecutionResult(ExecutionResult {
            exit_code: 0,
            stdout_len: 1024,
            stderr_len: 0,
            stdout: "stdout".to_string(),
            stderr: "stderr".to_string(),
        });

        let message_id = 45;
        let encoded = encode_message(&message, message_id).unwrap();

        let mut cursor = std::io::Cursor::new(encoded);
        let (decoded, decoded_id) = decode_message(&mut cursor).unwrap();

        assert_eq!(message, decoded);
        assert_eq!(message_id, decoded_id);
    }

    #[test]
    fn test_encode_decode_signal() {
        let message = Message::Signal(Signal {
            session_id: 12345,
            signal: 2, // SIGINT
        });

        let message_id = 46;
        let encoded = encode_message(&message, message_id).unwrap();

        let mut cursor = std::io::Cursor::new(encoded);
        let (decoded, decoded_id) = decode_message(&mut cursor).unwrap();

        assert_eq!(message, decoded);
        assert_eq!(message_id, decoded_id);
    }

    #[test]
    fn test_encode_decode_shutdown() {
        let message = Message::Shutdown(Shutdown { force: true });

        let message_id = 47;
        let encoded = encode_message(&message, message_id).unwrap();

        let mut cursor = std::io::Cursor::new(encoded);
        let (decoded, decoded_id) = decode_message(&mut cursor).unwrap();

        assert_eq!(message, decoded);
        assert_eq!(message_id, decoded_id);
    }

    #[test]
    fn test_message_too_large() {
        // Create a message with a very large payload
        let large_env: HashMap<String, String> = (0..100000)
            .map(|i| (format!("VAR_{}", i), "x".repeat(100)))
            .collect();

        let message = Message::SessionInit(SessionInit {
            working_dir: "/tmp".to_string(),
            env: large_env,
            args: vec![],
            stdin_mode: "inherit".to_string(),
        });

        let result = encode_message(&message, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_write_read_message() {
        let message = Message::ExecutionResult(ExecutionResult {
            exit_code: 0,
            stdout_len: 512,
            stderr_len: 0,
            stdout: "stdout".to_string(),
            stderr: "stderr".to_string(),
        });

        let message_id = 100;

        // Write to a buffer
        let mut buffer = Vec::new();
        write_message(&mut buffer, &message, message_id).unwrap();

        // Read from the buffer
        let mut cursor = std::io::Cursor::new(buffer);
        let (decoded, decoded_id) = read_message(&mut cursor).unwrap();

        assert_eq!(message, decoded);
        assert_eq!(message_id, decoded_id);
    }

    #[test]
    fn test_multiple_messages() {
        let messages = vec![
            (
                Message::SessionInit(SessionInit {
                    working_dir: "/tmp".to_string(),
                    env: HashMap::new(),
                    args: vec![],
                    stdin_mode: "inherit".to_string(),
                }),
                1,
            ),
            (
                Message::Execute(Execute {
                    session_id: 1,
                    command: "echo test".to_string(),
                }),
                2,
            ),
            (
                Message::ExecutionResult(ExecutionResult {
                    exit_code: 0,
                    stdout_len: 5,
                    stderr_len: 0,
                    stdout: "stdout".to_string(),
                    stderr: "stderr".to_string(),
                }),
                3,
            ),
        ];

        // Write all messages to a buffer
        let mut buffer = Vec::new();
        for (msg, id) in &messages {
            write_message(&mut buffer, msg, *id).unwrap();
        }

        // Read all messages back
        let mut cursor = std::io::Cursor::new(buffer);
        for (expected_msg, expected_id) in &messages {
            let (msg, id) = read_message(&mut cursor).unwrap();
            assert_eq!(*expected_msg, msg);
            assert_eq!(*expected_id, id);
        }
    }
}

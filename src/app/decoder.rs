use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tracing::error;

use super::request::Request;
use crate::app::request::Response;

pub(super) const MAX_REQUEST_SIZE: usize = 4096; // 4KB
pub(super) const MAX_RESPONSE_SIZE: usize = 1024; // 1KB
pub(super) const MSG_DELIMITER: u8 = b'\0';

/// A State machine to avoid reallocations and facilitate the I/O Stream
/// operations.
///
/// This struct is just a wrapper to manipulate the [`UnixStream`] and avoid
/// unecessary reallocations.
pub struct IPCConnection {
    stream: Option<UnixStream>,
    read_buf: Vec<u8>,
    write_buf: Vec<u8>,
}

impl IPCConnection {
    /// Create a new [`IPCConnection`] instance.
    pub fn new(stream: Option<UnixStream>) -> Self {
        Self {
            stream,
            read_buf: Vec::with_capacity(MAX_REQUEST_SIZE),
            write_buf: Vec::with_capacity(MAX_RESPONSE_SIZE),
        }
    }

    /// Set the `stream` to send [`Request`] and receive [`Response`].
    #[inline]
    pub fn set_stream(&mut self, stream: UnixStream) {
        self.stream = Some(stream);
    }

    /// Read a [`Request`] from the `stream`.
    pub async fn read_request(&mut self) -> io::Result<Request> {
        let Some(stream) = self.stream.as_mut() else {
            error!("The stream provided is None.");

            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "No stream provided.".to_string(),
            ));
        };

        self.read_buf.clear();

        loop {
            if self.read_buf.len() >= MAX_REQUEST_SIZE {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "The request exceeded the size limit. Max size: {}",
                        MAX_REQUEST_SIZE
                    ),
                ));
            }

            let readed_bytes = stream.read_buf(&mut self.read_buf).await?;

            if readed_bytes == 0 {
                return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
            }

            if let Some(pos) = self.read_buf.iter().position(|&b| b == MSG_DELIMITER) {
                // return the msg without the delimiter
                return serde_json::from_slice(&self.read_buf[..pos])
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e));
            }
        }
    }

    /// Send a [`Response`] by the `stream`.
    pub async fn write_response(&mut self, response: Response) -> io::Result<()> {
        let Some(stream) = self.stream.as_mut() else {
            error!("The stream provided is None.");

            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "No stream provided.".to_string(),
            ));
        };

        response.to_buf_json(&mut self.write_buf).map_err(|e| {
            error!("Fail to converts the response to a json: {}", e.to_string());

            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Fail to converts the response to a json: {}", e.to_string()),
            )
        })?;

        stream.write_all(&self.write_buf).await
    }
}

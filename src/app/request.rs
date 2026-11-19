use serde::{Deserialize, Serialize};

use crate::app::decoder::MSG_DELIMITER;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(tag = "action", content = "payload", rename_all = "snake_case")]
pub enum Request {
    Unlock { user: String },
    Enroll { user: String },
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Response {
    Accept,
    Reject {
        reason: RejectReason,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum RejectReason {
    Busy,
    Timeout,
    MaxAttempts,
    NoCamFound,
    InternalError,
}

impl Response {
    /// Create a new [`Response::Accept`].
    pub fn accept() -> Self {
        Response::Accept
    }

    /// Create a new [`Response::Reject`].
    pub fn reject(reason: RejectReason, detail: Option<String>) -> Self {
        Self::Reject { reason, detail }
    }

    /// Converts the response to a json in a [`Vec<u8>`] of bytes.
    ///
    /// # Cautions
    ///
    /// This methods automatic adds the `MSG_DELIMITER` in final byte.
    pub fn to_bytes_json(&self) -> Result<Vec<u8>, serde_json::Error> {
        let mut buf = serde_json::to_vec(self)?;
        buf.push(super::decoder::MSG_DELIMITER);

        Ok(buf)
    }

    /// Converts the [`Response`] to bytes in a buffer [`Vec<u8>`].
    ///
    /// Will clear the buffer and will coverts the [`Response`] into bytes into
    /// buffer.
    ///
    /// # Cautions
    ///
    /// This methods automatic adds the `MSG_DELIMITER` in final byte.
    pub fn to_buf_json(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<(), serde_json::Error> {
        buf.clear();

        serde_json::to_writer(&mut *buf, self)?;
        buf.push(MSG_DELIMITER);

        Ok(())
    }
}

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::connection::MSG_DELIMITER;
use super::task::Task;

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

    /// Converts the response to a JSON in a [`Vec<u8>`] of bytes.
    ///
    /// # Cautions
    ///
    /// This method automatically appends MSG_DELIMITER as the final byte.
    pub fn to_bytes_json(&self) -> Result<Vec<u8>, serde_json::Error> {
        let mut buf = serde_json::to_vec(self)?;
        buf.push(MSG_DELIMITER);

        Ok(buf)
    }

    /// Converts the [`Response`] to bytes in a buffer [`Vec<u8>`].
    ///
    /// Clears the buffer and serializes [`Response`] into it as bytes.
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

/// Runs a job while responding to incoming [`Task`]s from `task_receiver` with
/// a fallback `response`.
pub(crate) async fn run_while_responding<F, T>(
    job: F,
    task_receiver: &mut mpsc::Receiver<Task>,
    response: Response,
) -> T
where
    F: std::future::Future<Output = T>,
{
    tokio::pin!(job);

    let mut channel_is_open = !task_receiver.is_closed();

    loop {
        tokio::select! {
            res = &mut job => return res,

            // Avoid checks when the channel is closed.
            // This optimization avoids busy-waiting.
            task_opt = task_receiver.recv(), if channel_is_open => {
                match task_opt {
                    Some(task) => {
                        let response_clone = response.clone();
                        let _ = task.reply_to.send(response_clone);
                    },
                    None => {
                        channel_is_open = false;
                    }
                }

            }
        }
    }
}

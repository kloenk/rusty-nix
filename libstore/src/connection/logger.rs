use std::sync::{Arc, RwLock};

use crate::error::ConnectionError;
use tokio::io::AsyncWriteExt;
use tokio::net::unix::WriteHalf;

pub const STDERR_LAST: [u8; 4] = [0x73, 0x74, 0x6c, 0x61];

pub enum WorkFinish {
    Done,
    Error(String, usize),
}

pub const WorkDone: WorkFinish = WorkFinish::Done;

pub struct TunnelLogger<'a> {
    pub can_send_stderr: bool,
    pub pending_msgs: Vec<String>,

    pub client_version: u16,

    writer: Arc<RwLock<WriteHalf<'a>>>,
}

impl<'a> TunnelLogger<'a> {
    pub fn new(client_version: u16, writer: Arc<RwLock<WriteHalf<'a>>>) -> Self {
        Self {
            client_version,
            writer,

            can_send_stderr: false,
            pending_msgs: Vec::new(),
        }
    }

    pub async fn start_work(&mut self) -> Result<(), ConnectionError> {
        self.can_send_stderr = true;

        let mut writer = self.writer.write().unwrap(); // TODO: do we need error handling?
        for v in self.pending_msgs.drain(..) {
            writer.write(&v.as_ref()).await?;
        }
        writer.flush().await?;

        Ok(())
    }

    pub async fn stop_work(&mut self, state: WorkFinish) -> Result<(), ConnectionError> {
        self.can_send_stderr = false;

        let mut writer = self.writer.write().unwrap(); // TODO: error handling?
        if let WorkFinish::Error(v, s) = state {
            unimplemented!();
        } else {
            writer.write(&STDERR_LAST).await?;
            writer.write(&[0, 0, 0, 0]).await?;
        }

        writer.flush().await?;

        Ok(())
    }
}

use std::sync::{Arc, RwLock};

use crate::error::ConnectionError;
use tokio::io::AsyncWriteExt;
use tokio::net::unix::WriteHalf;

use byteorder::{ByteOrder, LittleEndian};

pub const STDERR_NEXT: u64 = 0x6f6c6d67;
pub const STDERR_READ: u64 = 0x64617461;
pub const STDERR_WRITE: u64 = 0x64617416;
pub const STDERR_LAST: u64 = 0x616c7473;
pub const STDERR_ERROR: u64 = 0x63787470;
pub const STDERR_START_ACTIVITY: u64 = 0x53545254;
pub const STDERR_STOP_ACTIVITY: u64 = 0x53544f50;
pub const STDERR_RESULT: u64 = 0x52534c54;

pub enum WorkFinish {
    Done,
    Error(String, usize),
}

pub const WORKDONE: WorkFinish = WorkFinish::Done;

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
        let mut buf: [u8; 8] = [0; 8];
        if let WorkFinish::Error(msg, s) = state {
            LittleEndian::write_u64(&mut buf, STDERR_ERROR);
            writer.write(&buf).await?;
            writer.write(&msg.as_ref()).await?;
            if s != 0 {
                LittleEndian::write_u64(&mut buf, s as u64);
                writer.write(&buf).await?;
            }
        } else {
            LittleEndian::write_u64(&mut buf, STDERR_LAST);
            writer.write(&buf).await?;
        }

        writer.flush().await?;

        Ok(())
    }
}

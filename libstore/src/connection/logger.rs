use std::sync::{RwLock, Arc};


use crate::error::ConnectionError;
use tokio::net::unix::WriteHalf;
use tokio::io::AsyncWriteExt;

pub struct TunnelLogger<'a> {
    
    pub can_send_stderr: bool,
    pub pending_msgs: Vec<String>,

    pub client_version: u16,

    writer: Arc<RwLock<WriteHalf<'a>>>,
}

impl<'a> TunnelLogger<'a> {
    pub fn new(client_version: u16, writer: Arc<RwLock<WriteHalf<'a>>>) -> Self {
        Self {
            client_version, writer,

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
}
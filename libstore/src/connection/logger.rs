

pub struct TunnelLogger {
    
    pub can_send_stderr: bool,
    pub pending_msgs: Vec<String>,

    pub client_version: usize,
}
pub const WORKER_MAGIC_1: u32 = 0x6e697863;
pub const WORKER_MAGIC_2: u32 = 0x6478696f;
pub const PROTOCOL_VERSION: u16 = 0x115;

pub struct Connection {
    pub trusted: bool,
    pub allowed: bool,
}

impl Connection {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Connection {
    fn default() -> Self {
        Self {
            trusted: false,
            allowed: false,
        }
    }
}

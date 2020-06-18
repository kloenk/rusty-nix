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

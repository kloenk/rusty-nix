use std::os::unix::net::UnixStream;

use libstore::error::CommandResult;
use libutil::config::NixConfig;

pub struct Config {
    pub stdio: bool,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(unused_must_use)]
    pub fn run(self, config: &NixConfig) -> CommandResult<()> {
        if self.stdio {
            // implement stdio for other store types
            let socket_path = "/nix/var/nix/daemon-socket/socket"; // FIXME: read from config
            let stream = UnixStream::connect(socket_path)?;

            let socket_arc = std::sync::Arc::new(stream);
            let (mut socket_tx, mut socket_rx) = (socket_arc.try_clone()?, socket_arc.try_clone()?);

            use std::io::{copy, stdin, stdout};
            use std::thread::spawn;
            let connections = vec![
                spawn(move || copy(&mut stdin(), &mut socket_tx)),
                spawn(move || copy(&mut socket_rx, &mut stdout())),
            ];

            for t in connections {
                t.join().unwrap();
            }
        }
        println!("{:#?}", config);
        Ok(()) // FIXME: unreachable() //?
    }
}

impl Default for Config {
    fn default() -> Self {
        Self { stdio: false }
    }
}

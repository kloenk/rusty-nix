use std::sync::Arc;

use tokio::io;
use tokio::net::UnixStream;

#[macro_use]
extern crate log;
use clap::{App, Arg};

use error::CommandResult;
use libutil::config::NixConfig;

pub mod error;

pub struct NixDaemon {
    pub stdio: bool,
    pub nix_config: Arc<NixConfig>,
}

impl NixDaemon {
    pub async fn new() -> CommandResult<Self> {
        let mut app = App::new(env!("CARGO_PKG_NAME"))
            .version(env!("CARGO_PKG_VERSION"))
            .author(env!("CARGO_PKG_AUTHORS"))
            .about(env!("CARGO_PKG_DESCRIPTION"))
            .arg(
                Arg::with_name("daemon")
                    .long("daemon")
                    .help("ignored for backwards compability")
                    .takes_value(false),
            )
            .arg(
                Arg::with_name("stdio")
                    .long("stdio")
                    .help("read from stdin")
                    .takes_value(false),
            )
            .arg(
                Arg::with_name("config")
                    .long("config")
                    .short("c")
                    .help("set nix conifg file")
                    .takes_value(true)
                    .default_value("/etc/nix/nix.conf"),
            );
        // FIXME: add all other options

        if cfg!(feature = "color") {
            app = app
                .setting(clap::AppSettings::ColorAuto)
                .setting(clap::AppSettings::ColoredHelp);
        }

        let matches = app.get_matches();

        let config_file = std::path::Path::new(matches.value_of("config").unwrap());
        let nix_config = libutil::config::NixConfig::parse_file(config_file)?;
        // TODO: merge with args

        let mut config = Self::new_from_config(Arc::new(nix_config));

        if matches.is_present("daemon") {
            trace!("provided `--daemon` which is only here for backward compability");
        }

        if matches.is_present("stdio") {
            trace!("running in stdio mode");
            config.stdio = true;
        }

        Ok(config)
    }

    #[allow(unused_must_use)]
    pub async fn run(self) -> CommandResult<()> {
        if self.stdio {
            // implement stdio for other store types
            /*let stream = UnixStream::connect(&self.nix_config.nix_daemon_socket_file)?;

            let socket_arc = std::sync::Arc::new(stream);
            let (mut socket_tx, mut socket_rx) = (socket_arc.try_clone()?, socket_arc.try_clone()?);

            use std::io::{copy, stdin, stdout};
            use std::thread::spawn;
            let connections = vec![ // This is broken. Rewrite to async?
                spawn(move || loop { copy(&mut stdin(), &mut socket_tx); } ),
                spawn(move || loop { copy(&mut socket_rx, &mut stdout()); } ),
            ];

            for t in connections {
                t.join().unwrap();
            }*/
            /*let socket_file = &self.nix_config.nix_daemon_socket_file;
            debug!("stdio: connecting to socket {}", socket_file);

            let mut stream = UnixStream::connect(socket_file).await?;
            let (mut read, mut write) = stream.split();

            let mut stdin = io::stdin();
            let mut stdout = io::stdout();

            warn!("copying");

            //let join = futures::future::join_all(connections).await;
            let join = futures::future::join(io::copy(&mut stdin, &mut write), io::copy(&mut read, &mut stdout)).await;

            warn!("join: ${:?}", join);*/
            unimplemented!();
        } else {
            self.daemon_loop().await?;
        }
        Ok(()) // FIXME: unreachable() //?
    }

    async fn daemon_loop(self) -> CommandResult<()> {
        println!("running loop");

        Ok(())
    }

    fn new_from_config(config: Arc<NixConfig>) -> Self {
        Self {
            stdio: false,
            nix_config: config,
        }
    }
}

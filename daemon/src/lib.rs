use std::convert::TryInto;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::stream::StreamExt;

use libstore::connection::Connection;

#[macro_use]
extern crate log;
use clap::{App, Arg};

use error::CommandResult;

pub mod error;

pub struct NixDaemon {
    pub stdio: bool,
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
        let mut store_config = libstore::CONFIG.write().unwrap();
        *store_config = nix_config;
        drop(store_config);
        // TODO: merge with args

        let mut config = Self { stdio: false };

        if matches.is_present("daemon") {
            info!("provided `--daemon` which is only here for backward compability");
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
        std::env::set_current_dir("/")?;

        // TODO: get rid of zombies

        #[allow(unused_assignments)]
        let mut listener: Option<UnixListener> = None;

        if let Ok(listen_fds) = std::env::var("LISTEN_FDS") {
            let fd: i32 = listen_fds.parse().unwrap();
            let raw_fd: std::os::unix::net::UnixListener =
                unsafe { std::os::unix::io::FromRawFd::from_raw_fd(fd) };
            listener = UnixListener::from_std(raw_fd).ok();
        } else {
            let config = libstore::CONFIG.read().unwrap();
            let file = config.nix_daemon_socket_file.to_string();
            drop(config);
            info!("listening on {}", file);
            // TODO: create dirs
            listener = Some(UnixListener::bind(&file)?);

            // set permissions
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o666);
            std::fs::set_permissions(&file, perms)?;

            // TODO: exit trap to remove socket
        }

        let mut listener = listener.expect("there is no listener");

        while let Some(stream) = listener.next().await {
            match stream {
                Ok(stream) => {
                    if let Err(e) = self.handle_connection(stream).await {
                        // FIXME: multithreading!!!
                        // TODO: print errors
                        warn!("{}", e);
                    }
                }
                Err(e) => {
                    warn!("Error accepting connection: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn handle_connection(&self, stream: UnixStream) -> CommandResult<()> {
        let mut stream = stream;

        let creds = stream.peer_cred()?;

        //let user = users::get_user_by_uid(creds.uid);
        let user = match users::get_user_by_uid(creds.uid) {
            Some(v) => v.name().to_string_lossy().to_string(),
            None => "not allowed user".to_string(),
        };
        let group = match users::get_group_by_gid(creds.gid) {
            Some(v) => v.name().to_string_lossy().to_string(),
            None => "not allowed group".to_string(),
        };

        let config = libstore::CONFIG.read().unwrap();
        let trusted = config.is_trusted_user(&user, &group);

        if !config.is_allowed_user(&user, &group) {
            return Err(crate::error::CommandError::DisallowedUser { user });
        }
        let store = config.store.to_string();
        drop(config);

        info!(
            "accepted connection from user {}{}",
            user,
            if trusted { " (trusted)" } else { "" },
            //if let Some(pid) = creds.pid { format!(" pid: {}", pid) } else { "".to_string() }
        ); // TODO: pid

        // verify client version
        let mut buffer: [u8; 10] = [0; 10];

        stream.read(&mut buffer[..]).await?;

        let magic = u32::from_le_bytes(buffer[0..4].try_into().unwrap());
        if magic != libstore::connection::WORKER_MAGIC_1 {
            return Err(crate::error::CommandError::InvalidMagic {});
        }

        let magic = u32::to_le_bytes(libstore::connection::WORKER_MAGIC_2);
        let version = u16::to_le_bytes(libstore::connection::PROTOCOL_VERSION);
        stream.write(&magic).await?;
        stream.write(&[0, 0, 0, 0]).await?;
        stream.write(&version).await?;
        stream.write(&[0, 0, 0, 0, 0, 0]).await?;

        stream.read(&mut buffer[..]).await?;
        let version = u16::from_le_bytes(buffer[0..2].try_into().unwrap());
        if version < 0x10a {
            return Err(crate::error::CommandError::InvalidVersion {});
        }

        stream.read(&mut buffer[..]).await?;
        stream.read(&mut buffer[..]).await?;

        trace!("version an client matching");

        //let params = std::collections::HashMap::new();
        // TODO: override settings via Params

        //let store = libstore::open_store_build(&store, params).await.unwrap();

        let con = libstore::source::Connection::new(stream);

        let connection = Connection::new(trusted, version, con, &store, creds.uid, user)
            .await
            .unwrap();

        #[allow(clippy::single_match)] // TODO: add magic?
        match connection.run().await {
            // TODO:
            // FIXME: error
            Err(e) => {
                trace!("shutting down stream");
                info!("got error {} from daemon loop", e);
                //stream.shutdown(std::net::Shutdown::Both)?; // TODO: where to shutndown
                //Err(e);
            }
            Ok(_) => {}
        }

        Ok(())
    }
}

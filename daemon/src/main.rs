#[macro_use]
extern crate log;
extern crate env_logger;
use clap::{App, Arg};

fn main() {
    /*if let Err(_) = std::env::var("RUST_LOG") {
        // println!("set log env var");
        std::env::set_var("RUST_LOG", "nix-daemon=trace,trace"); // TODO: change on release?
    }*/
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "nix-daemon=trace,main=trace"); // TODO: change on release?
    }
    env_logger::init();
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

    let mut config = nix_daemon::Config::new();

    let config_file = std::path::Path::new(matches.value_of("config").unwrap());
    let nix_config = libutil::config::NixConfig::parse_file(config_file).unwrap();
    // TODO: merge with args

    if matches.is_present("daemon") {
        trace!("provided `--daemon` which is only here for backward compability");
    }

    if matches.is_present("stdio") {
        trace!("running in stdio mode");
        config.stdio = true;
    }

    match config.run(&nix_config) {
        Ok(_) => std::process::exit(0),
        Err(v) => {
            error!("{}", v);
            std::process::exit(v.get_code()); // TODO: change exit codes
        }
    }
}

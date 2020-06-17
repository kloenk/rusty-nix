#[macro_use]
extern crate log;
extern crate env_logger;

use nix_daemon::error::CommandResult;
use nix_daemon::NixDaemon;

fn main() {
    // setup env
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "nix-daemon=trace,main=trace"); // TODO: change on release?
    }
    env_logger::init();

    // start app
    if let Err(e) = run() {
        error!("{}", e);
        std::process::exit(e.get_code());
    }
}

#[tokio::main]
async fn run() -> CommandResult<()> {
    let daemon = NixDaemon::new().await?;
    daemon.run().await?;
    Ok(())
}

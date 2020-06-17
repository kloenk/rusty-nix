#[macro_use]
extern crate log;
extern crate env_logger;

use nix_daemon::error::CommandResult;
use nix_daemon::NixDaemon;

fn main() {
    // setup env
    env_logger::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // start app
    if let Err(e) = run() {
        error!("{}", e);
        //println!("error: {}", e);
        std::process::exit(e.get_code());
    }
}

#[tokio::main]
async fn run() -> CommandResult<()> {
    let daemon = NixDaemon::new().await?;
    daemon.run().await?;
    Ok(())
}

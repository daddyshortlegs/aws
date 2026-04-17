use clap::{Parser, Subcommand};

mod client;
mod cmd;

#[derive(Parser)]
#[command(name = "andy-cli", about = "CLI for Andy's Web Services")]
struct Cli {
    /// Proxy base URL
    #[arg(
        long,
        env = "ANDYWS_ENDPOINT",
        default_value = "http://127.0.0.1:8080",
        global = true
    )]
    proxy: String,

    /// Output raw JSON instead of formatted tables
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage virtual machines
    Vm {
        #[command(subcommand)]
        action: cmd::vm::VmCommand,
    },
    /// Manage volumes
    Volume {
        #[command(subcommand)]
        action: cmd::volume::VolumeCommand,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let client = client::Client::new(cli.proxy);

    let result = match cli.command {
        Command::Vm { action } => cmd::vm::run(action, &client, cli.json).await,
        Command::Volume { action } => cmd::volume::run(action, &client, cli.json).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

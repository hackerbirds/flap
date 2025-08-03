use std::fs::File;

use clap::{Parser, Subcommand};
use flap_lib::{receiver::P2pReceiver, sender::P2pSender};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Adds files to myapp
    Send {
        file_path: String,
    },
    Receive {
        ticket_string: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match cli.command {
        Commands::Send { file_path } => {
            let sender = P2pSender::new().await.unwrap();

            let file = File::open(file_path).unwrap();
            let ticket = sender.send(file).await.unwrap();

            println!("File is ready. The ticket is: {}", ticket.convert());

            tokio::signal::ctrl_c().await.unwrap();
        }
        Commands::Receive { ticket_string } => {
            let receiver = P2pReceiver::new().await.unwrap();
            let ticket = ticket_string.parse().unwrap();
            let _retrieved_bytes = receiver.retrieve(ticket).await.unwrap();
        }
    }
}

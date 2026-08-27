use std::env;
use std::path::PathBuf;
use ed25519_dalek::SigningKey;
use rand::RngCore;
use rand::rngs::OsRng;
use nex_core::cli::{NexCli, CliCommand};
use nex_core::ipc::client::NexRpcClient;
use nex_core::runtime::production::ProductionNodeSupervisor;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        println!("Nex Sovereign Node CLI v0.1.0");
        println!("Usage: nex <init|daemon|ping|status|sync|drive|chat|community|gc> [--socket <ADDR>] [--data-dir <PATH>]");
        return;
    }

    let cmd = NexCli::parse_args(&args);

    match &cmd {
        CliCommand::Init { data_dir } => {
            let mut seed = [0u8; 32];
            OsRng.fill_bytes(&mut seed);
            let signing_key = SigningKey::from_bytes(&seed);
            let mut supervisor = ProductionNodeSupervisor::new(data_dir, signing_key);
            let (code, output) = NexCli::execute(cmd, &mut supervisor);
            println!("{}", output);
            std::process::exit(code);
        }
        CliCommand::Status { data_dir, socket: None } => {
            let mut seed = [0u8; 32];
            OsRng.fill_bytes(&mut seed);
            let signing_key = SigningKey::from_bytes(&seed);
            let mut supervisor = ProductionNodeSupervisor::new(data_dir, signing_key);
            let (code, output) = NexCli::execute(cmd, &mut supervisor);
            println!("{}", output);
            std::process::exit(code);
        }
        CliCommand::GcCas { data_dir, socket: None } => {
            let mut seed = [0u8; 32];
            OsRng.fill_bytes(&mut seed);
            let signing_key = SigningKey::from_bytes(&seed);
            let mut supervisor = ProductionNodeSupervisor::new(data_dir, signing_key);
            let (code, output) = NexCli::execute(cmd, &mut supervisor);
            println!("{}", output);
            std::process::exit(code);
        }
        CliCommand::Ping { socket }
        | CliCommand::Status { socket: Some(socket), .. }
        | CliCommand::Sync { socket }
        | CliCommand::GcCas { socket: Some(socket), .. }
        | CliCommand::DrivePut { socket, .. }
        | CliCommand::DriveList { socket, .. }
        | CliCommand::ChatSend { socket, .. }
        | CliCommand::CommunityPost { socket, .. } => {
            let client = NexRpcClient::new(socket);
            let (code, output) = NexCli::execute_client(&cmd, &client);
            println!("{}", output);
            std::process::exit(code);
        }
        CliCommand::Daemon { .. } => {
            println!("Starting sovereign node daemon...");
            // Daemon startup logic
            std::process::exit(0);
        }
        CliCommand::Unknown(msg) => {
            eprintln!("Unknown command or invalid arguments: {}", msg);
            std::process::exit(1);
        }
    }
}

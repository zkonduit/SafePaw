use std::env;
use std::sync::Arc;

use anyhow::bail;
use safepaw::agent::LocalAgentManager;
use safepaw::cli::{VmMode, build_cli, resolve_vm_mode, run_agent_subcommand, run_vm_subcommand};
use safepaw::vm::{LocalVmApi, MultipassCli, TokioCommandExecutor};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() {
    // Initialize tracing subscriber with environment filter
    // Can be controlled via RUST_LOG env var (e.g., RUST_LOG=debug)
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("safepaw=info")))
        .init();

    if let Err(err) = run().await {
        eprintln!("error: {err}");
        for cause in err.chain().skip(1) {
            eprintln!("caused by: {cause}");
        }
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    if env::args_os().nth(1).is_none() {
        let mut cli = build_cli();
        cli.print_help().expect("failed to print help");
        println!();
        return Ok(());
    }

    let matches = build_cli().get_matches();

    match matches.subcommand() {
        Some(("start", start_matches)) => {
            let host = start_matches
                .get_one::<String>("host")
                .map(String::as_str)
                .unwrap_or("0.0.0.0");
            let ui_port = *start_matches.get_one::<u16>("ui-port").unwrap_or(&8888);
            let api_port = *start_matches.get_one::<u16>("api-port").unwrap_or(&8889);

            let multipass = Arc::new(MultipassCli::new(TokioCommandExecutor));
            let vm_api =
                Arc::new(LocalVmApi::new(multipass.clone())) as Arc<dyn safepaw::vm::VmApi>;
            let agent_manager = Arc::new(LocalAgentManager::new(vm_api.clone())?)
                as Arc<dyn safepaw::agent::AgentManager>;

            safepaw::server::run_server(vm_api, agent_manager, host, ui_port, api_port).await?;
        }
        Some(("vm", vm_matches)) => match resolve_vm_mode(vm_matches)? {
            VmMode::Local => {
                let multipass = Arc::new(MultipassCli::new(TokioCommandExecutor));
                let api = LocalVmApi::new(multipass);
                let lines = run_vm_subcommand(vm_matches, &api).await?;
                for line in lines {
                    println!("{line}");
                }
            }
            VmMode::Network => {
                bail!("network mode is planned but not implemented yet");
            }
        },
        Some(("agent", agent_matches)) => {
            let multipass = Arc::new(MultipassCli::new(TokioCommandExecutor));
            let vm_api = Arc::new(LocalVmApi::new(multipass.clone()));
            let agent_manager = LocalAgentManager::new(vm_api)?;
            let lines = run_agent_subcommand(agent_matches, &agent_manager).await?;
            for line in lines {
                println!("{line}");
            }
        }
        _ => {}
    }

    Ok(())
}

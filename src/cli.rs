use std::sync::Arc;

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use clap::{Arg, ArgMatches, Command};
use tracing::info;

use crate::vm::{Multipass, VmStatusResponse, VmSummary};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmMode {
    Local,
    Network,
}

#[async_trait]
pub trait VmApi: Send + Sync {
    async fn launch(&self, name: &str) -> Result<()>;
    async fn start(&self, name: &str) -> Result<()>;
    async fn stop(&self, name: &str) -> Result<()>;
    async fn restart(&self, name: &str) -> Result<()>;
    async fn delete(&self, name: &str) -> Result<()>;
    async fn info(&self, name: &str) -> Result<VmStatusResponse>;
    async fn list(&self) -> Result<Vec<VmSummary>>;
}

#[derive(Clone)]
pub struct LocalVmApi {
    multipass: Arc<dyn Multipass>,
}

impl LocalVmApi {
    pub fn new(multipass: Arc<dyn Multipass>) -> Self {
        Self { multipass }
    }
}

#[async_trait]
impl VmApi for LocalVmApi {
    async fn launch(&self, name: &str) -> Result<()> {
        info!(vm_name = name, "launching VM. This may take a couple of minutes.");
        self.multipass
            .launch(name)
            .await
            .with_context(|| format!("failed to launch VM {name}"))?;
        info!(vm_name = name, "VM launched successfully");
        Ok(())
    }

    async fn start(&self, name: &str) -> Result<()> {
        info!(vm_name = name, "starting VM");
        self.multipass
            .start(name)
            .await
            .with_context(|| format!("failed to start VM {name}"))?;
        info!(vm_name = name, "VM started successfully");
        Ok(())
    }

    async fn stop(&self, name: &str) -> Result<()> {
        info!(vm_name = name, "stopping VM");
        self.multipass
            .stop(name)
            .await
            .with_context(|| format!("failed to stop VM {name}"))?;
        info!(vm_name = name, "VM stopped successfully");
        Ok(())
    }

    async fn restart(&self, name: &str) -> Result<()> {
        info!(vm_name = name, "restarting VM");
        self.multipass
            .restart(name)
            .await
            .with_context(|| format!("failed to restart VM {name}"))?;
        info!(vm_name = name, "VM restarted successfully");
        Ok(())
    }

    async fn delete(&self, name: &str) -> Result<()> {
        info!(vm_name = name, "deleting VM");
        self.multipass
            .delete(name)
            .await
            .with_context(|| format!("failed to delete VM {name}"))?;
        info!(vm_name = name, "VM deleted successfully");
        Ok(())
    }

    async fn info(&self, name: &str) -> Result<VmStatusResponse> {
        info!(vm_name = name, "getting VM info");
        self.multipass
            .info(name)
            .await
            .with_context(|| format!("failed to get info for VM {name}"))
    }

    async fn list(&self) -> Result<Vec<VmSummary>> {
        info!("listing VMs");
        self.multipass
            .list()
            .await
            .context("failed to list VMs from multipass")
    }
}

pub fn build_cli() -> Command {
    Command::new("safepaw")
        .about("Agents for the paranoid.")
        .long_about("SafePaw orchestrates isolated agent runtimes backed by Multipass VMs.")
        .subcommand(
            Command::new("vm")
                .about("Manage VM lifecycle through multipass")
                .arg(
                    Arg::new("mode")
                        .long("mode")
                        .value_name("MODE")
                        .value_parser(["local", "network"])
                        .global(true)
                        .default_value("local")
                        .help("Execution mode: local (default) or network (planned)"),
                )
                .subcommand_required(true)
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("launch")
                        .about("Launch a new VM")
                        .arg(Arg::new("name").required(true).help("VM name to create")),
                )
                .subcommand(
                    Command::new("start")
                        .about("Start a stopped VM")
                        .arg(Arg::new("name").required(true).help("VM name to start")),
                )
                .subcommand(
                    Command::new("stop")
                        .about("Stop a running VM")
                        .arg(Arg::new("name").required(true).help("VM name to stop")),
                )
                .subcommand(
                    Command::new("restart")
                        .about("Restart a VM")
                        .arg(Arg::new("name").required(true).help("VM name to restart")),
                )
                .subcommand(
                    Command::new("delete")
                        .about("Delete a VM permanently")
                        .arg(Arg::new("name").required(true).help("VM name to delete")),
                )
                .subcommand(
                    Command::new("info")
                        .about("Get detailed VM information")
                        .arg(Arg::new("name").required(true).help("VM name to inspect")),
                )
                .subcommand(Command::new("list").about("List all VMs")),
        )
}

pub fn resolve_vm_mode(matches: &ArgMatches) -> Result<VmMode> {
    let mode = matches
        .get_one::<String>("mode")
        .map(String::as_str)
        .unwrap_or("local");

    match mode {
        "local" => Ok(VmMode::Local),
        "network" => Ok(VmMode::Network),
        _ => bail!("unsupported vm mode: {mode}"),
    }
}

fn format_vm_summary(vm: &VmSummary) -> String {
    let mut parts = vec![vm.name.clone(), vm.state.clone()];

    if let Some(ref ipv4_addrs) = vm.ipv4 {
        if !ipv4_addrs.is_empty() {
            parts.push(ipv4_addrs.join(","));
        }
    }

    if let Some(ref release) = vm.release {
        parts.push(release.clone());
    }

    parts.join(" | ")
}

fn format_vm_info(info: &VmStatusResponse) -> Vec<String> {
    let mut lines = vec![
        format!("Name:  {}", info.name),
        format!("State: {}", info.state),
    ];

    if let Some(ref ipv4_addrs) = info.ipv4 {
        if !ipv4_addrs.is_empty() {
            lines.push(format!("IPv4:  {}", ipv4_addrs.join(", ")));
        }
    }

    if let Some(ref release) = info.release {
        lines.push(format!("Release: {}", release));
    }

    if let Some(ref image_release) = info.image_release {
        lines.push(format!("Image:   {}", image_release));
    }

    if let Some(ref cpus) = info.cpu_count {
        lines.push(format!("CPUs:  {}", cpus));
    }

    if let (Some(total), Some(used)) = (info.memory_total, info.memory_used) {
        let total_mb = total / 1024 / 1024;
        let used_mb = used / 1024 / 1024;
        let percent = (used as f64 / total as f64 * 100.0) as u64;
        lines.push(format!("Memory: {} MiB / {} MiB ({}%)", used_mb, total_mb, percent));
    }

    if let (Some(total), Some(used)) = (info.disk_total, info.disk_used) {
        let total_gb = total / 1024 / 1024 / 1024;
        let used_gb = used / 1024 / 1024 / 1024;
        let percent = (used as f64 / total as f64 * 100.0) as u64;
        lines.push(format!("Disk:   {} GiB / {} GiB ({}%)", used_gb, total_gb, percent));
    }

    lines
}

pub async fn run_vm_subcommand(matches: &ArgMatches, api: &dyn VmApi) -> Result<Vec<String>> {
    match matches.subcommand() {
        Some(("launch", launch_matches)) => {
            let name = required_arg(launch_matches, "name")?;
            api.launch(name).await?;
            Ok(vec![format!("launched {name}")])
        }
        Some(("start", start_matches)) => {
            let name = required_arg(start_matches, "name")?;
            api.start(name).await?;
            Ok(vec![format!("started {name}")])
        }
        Some(("stop", stop_matches)) => {
            let name = required_arg(stop_matches, "name")?;
            api.stop(name).await?;
            Ok(vec![format!("stopped {name}")])
        }
        Some(("restart", restart_matches)) => {
            let name = required_arg(restart_matches, "name")?;
            api.restart(name).await?;
            Ok(vec![format!("restarted {name}")])
        }
        Some(("delete", delete_matches)) => {
            let name = required_arg(delete_matches, "name")?;
            api.delete(name).await?;
            Ok(vec![format!("deleted {name}")])
        }
        Some(("info", info_matches)) => {
            let name = required_arg(info_matches, "name")?;
            let info = api.info(name).await?;
            Ok(format_vm_info(&info))
        }
        Some(("list", _)) => {
            let vms = api.list().await?;
            if vms.is_empty() {
                Ok(vec!["No VMs found".to_string()])
            } else {
                Ok(vms
                    .into_iter()
                    .map(|vm| format_vm_summary(&vm))
                    .collect())
            }
        }
        _ => Ok(Vec::new()),
    }
}

fn required_arg<'a>(matches: &'a ArgMatches, name: &str) -> Result<&'a str> {
    matches
        .get_one::<String>(name)
        .map(String::as_str)
        .with_context(|| format!("missing required argument: {name}"))
}

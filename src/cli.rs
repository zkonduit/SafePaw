use anyhow::{Context, Result, bail};
use clap::{Arg, ArgMatches, Command};

use crate::vm::{VmApi, VmStatusResponse, VmSummary, handlers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmMode {
    Local,
    Network,
}

pub fn build_cli() -> Command {
    Command::new("safepaw")
        .about("Agents for the paranoid.")
        .long_about("SafePaw orchestrates isolated agent runtimes backed by Multipass VMs.")
        .subcommand(
            Command::new("start")
                .about("Start SafePaw server daemon")
                .long_about("Starts the SafePaw UI server and REST API daemon")
                .arg(
                    Arg::new("host")
                        .long("host")
                        .value_name("HOST")
                        .default_value("0.0.0.0")
                        .help("Host address to bind servers (e.g., 0.0.0.0, 127.0.0.1, localhost)"),
                )
                .arg(
                    Arg::new("ui-port")
                        .long("ui-port")
                        .value_name("PORT")
                        .default_value("8888")
                        .value_parser(clap::value_parser!(u16))
                        .help("Port for the UI server"),
                )
                .arg(
                    Arg::new("api-port")
                        .long("api-port")
                        .value_name("PORT")
                        .default_value("8889")
                        .value_parser(clap::value_parser!(u16))
                        .help("Port for the REST API server"),
                ),
        )
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

    if let Some(ref ipv4_addrs) = vm.ipv4
        && !ipv4_addrs.is_empty()
    {
        parts.push(ipv4_addrs.join(","));
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

    if let Some(ref ipv4_addrs) = info.ipv4
        && !ipv4_addrs.is_empty()
    {
        lines.push(format!("IPv4:  {}", ipv4_addrs.join(", ")));
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
        lines.push(format!(
            "Memory: {} MiB / {} MiB ({}%)",
            used_mb, total_mb, percent
        ));
    }

    if let (Some(total), Some(used)) = (info.disk_total, info.disk_used) {
        let total_gb = total / 1024 / 1024 / 1024;
        let used_gb = used / 1024 / 1024 / 1024;
        let percent = (used as f64 / total as f64 * 100.0) as u64;
        lines.push(format!(
            "Disk:   {} GiB / {} GiB ({}%)",
            used_gb, total_gb, percent
        ));
    }

    lines
}

pub async fn run_vm_subcommand(matches: &ArgMatches, api: &dyn VmApi) -> Result<Vec<String>> {
    match matches.subcommand() {
        Some(("launch", launch_matches)) => {
            let name = required_arg(launch_matches, "name")?;
            let result = handlers::launch_vm(api, name).await;
            if result.success {
                Ok(vec![result.message])
            } else {
                Err(anyhow::anyhow!(result.message))
            }
        }
        Some(("start", start_matches)) => {
            let name = required_arg(start_matches, "name")?;
            let result = handlers::start_vm(api, name).await;
            if result.success {
                Ok(vec![result.message])
            } else {
                Err(anyhow::anyhow!(result.message))
            }
        }
        Some(("stop", stop_matches)) => {
            let name = required_arg(stop_matches, "name")?;
            let result = handlers::stop_vm(api, name).await;
            if result.success {
                Ok(vec![result.message])
            } else {
                Err(anyhow::anyhow!(result.message))
            }
        }
        Some(("restart", restart_matches)) => {
            let name = required_arg(restart_matches, "name")?;
            let result = handlers::restart_vm(api, name).await;
            if result.success {
                Ok(vec![result.message])
            } else {
                Err(anyhow::anyhow!(result.message))
            }
        }
        Some(("delete", delete_matches)) => {
            let name = required_arg(delete_matches, "name")?;
            let result = handlers::delete_vm(api, name).await;
            if result.success {
                Ok(vec![result.message])
            } else {
                Err(anyhow::anyhow!(result.message))
            }
        }
        Some(("info", info_matches)) => {
            let name = required_arg(info_matches, "name")?;
            let result = handlers::get_vm_info(api, name).await;
            if result.success {
                if let Some(info) = result.data {
                    Ok(format_vm_info(&info))
                } else {
                    Ok(vec![result.message])
                }
            } else {
                Err(anyhow::anyhow!(result.message))
            }
        }
        Some(("list", _)) => {
            let result = handlers::list_vms(api).await;
            if result.success {
                if let Some(vms) = result.data {
                    if vms.is_empty() {
                        Ok(vec!["No VMs found".to_string()])
                    } else {
                        Ok(vms.into_iter().map(|vm| format_vm_summary(&vm)).collect())
                    }
                } else {
                    Ok(vec![result.message])
                }
            } else {
                Err(anyhow::anyhow!(result.message))
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

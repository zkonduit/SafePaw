use anyhow::{Context, Result, bail};
use clap::{Arg, ArgMatches, Command};

use crate::agent::{
    AgentInstance, AgentManager, AgentType, OnboardAgentRequest, handlers as agent_handlers,
};
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
        .subcommand(
            Command::new("agent")
                .about("Manage agents within VMs")
                .arg_required_else_help(true)
                .subcommand_required(true)
                .subcommand(
                    Command::new("install")
                        .about("Install an agent in a VM")
                        .arg(
                            Arg::new("vm")
                                .long("vm")
                                .short('v')
                                .required(true)
                                .help("VM name where agent will be installed")
                                .long_help("VM name where agent will be installed. Use 'safepaw vm list' to see available VMs."),
                        )
                        .arg(
                            Arg::new("type")
                                .long("type")
                                .short('t')
                                .required(true)
                                .value_parser(["picoclaw"])
                                .help("Agent type to install"),
                        ),
                )
                .subcommand(
                    Command::new("onboard")
                        .about("Onboard (configure) an agent in a VM")
                        .long_about("Onboard (configure) an agent with LLM provider credentials. The agent must be installed first using 'safepaw agent install'.")
                        .arg(
                            Arg::new("vm")
                                .long("vm")
                                .short('v')
                                .required(true)
                                .help("VM name where agent will be onboarded")
                                .long_help("VM name where agent will be onboarded. Use 'safepaw vm list' to see available VMs."),
                        )
                        .arg(
                            Arg::new("type")
                                .long("type")
                                .short('t')
                                .required(true)
                                .value_parser(["picoclaw"])
                                .help("Agent type to onboard"),
                        )
                        .arg(
                            Arg::new("provider")
                                .long("provider")
                                .short('p')
                                .required(true)
                                .help("LLM provider (e.g., openrouter, anthropic, openai)"),
                        )
                        .arg(
                            Arg::new("model")
                                .long("model")
                                .short('m')
                                .help("Model name (e.g., openrouter/auto, claude-3-5-sonnet-20241022)"),
                        )
                        .arg(
                            Arg::new("name")
                                .long("name")
                                .short('n')
                                .help("Human-readable name for the agent"),
                        )
                        .arg(
                            Arg::new("api-key-name")
                                .long("api-key-name")
                                .short('k')
                                .default_value("openrouter-api-key")
                                .help("Name of API key in keychain/secrets manager"),
                        ),
                )
                .subcommand(
                    Command::new("list")
                        .about("List agents in a VM")
                        .arg(
                            Arg::new("vm")
                                .long("vm")
                                .short('v')
                                .required(true)
                                .help("VM name to list agents from")
                                .long_help("VM name to list agents from. Use 'safepaw vm list' to see available VMs."),
                        ),
                )
                .subcommand(
                    Command::new("get")
                        .about("Get agent details")
                        .arg(
                            Arg::new("vm")
                                .long("vm")
                                .short('v')
                                .required(true)
                                .help("VM name")
                                .long_help("VM name where the agent is running. Use 'safepaw vm list' to see available VMs."),
                        )
                        .arg(
                            Arg::new("agent-id")
                                .long("agent-id")
                                .short('a')
                                .required(true)
                                .help("Agent ID (UUID or name)")
                                .long_help("Agent ID (UUID or name). Use 'safepaw agent list --vm <VM_NAME>' to see agent IDs."),
                        ),
                )
                .subcommand(
                    Command::new("stop")
                        .about("Stop an agent")
                        .arg(
                            Arg::new("vm")
                                .long("vm")
                                .short('v')
                                .required(true)
                                .help("VM name")
                                .long_help("VM name where the agent is running. Use 'safepaw vm list' to see available VMs."),
                        )
                        .arg(
                            Arg::new("agent-id")
                                .long("agent-id")
                                .short('a')
                                .required(true)
                                .help("Agent ID (UUID or name)")
                                .long_help("Agent ID (UUID or name). Use 'safepaw agent list --vm <VM_NAME>' to see agent IDs."),
                        ),
                )
                .subcommand(
                    Command::new("delete")
                        .about("Delete an agent")
                        .arg(
                            Arg::new("vm")
                                .long("vm")
                                .short('v')
                                .required(true)
                                .help("VM name")
                                .long_help("VM name where the agent is running. Use 'safepaw vm list' to see available VMs."),
                        )
                        .arg(
                            Arg::new("agent-id")
                                .long("agent-id")
                                .short('a')
                                .required(true)
                                .help("Agent ID (UUID or name)")
                                .long_help("Agent ID (UUID or name). Use 'safepaw agent list --vm <VM_NAME>' to see agent IDs."),
                        ),
                )
                .subcommand(
                    Command::new("check")
                        .about("Check if an agent type is installed")
                        .arg(
                            Arg::new("vm")
                                .long("vm")
                                .short('v')
                                .required(true)
                                .help("VM name")
                                .long_help("VM name to check for agent installation. Use 'safepaw vm list' to see available VMs."),
                        )
                        .arg(
                            Arg::new("type")
                                .long("type")
                                .short('t')
                                .required(true)
                                .value_parser(["picoclaw"])
                                .help("Agent type to check"),
                        ),
                ),
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

pub async fn run_agent_subcommand(
    matches: &ArgMatches,
    agent_manager: &dyn AgentManager,
) -> Result<Vec<String>> {
    match matches.subcommand() {
        Some(("install", install_matches)) => {
            let vm_name = required_arg(install_matches, "vm")?;
            let agent_type_str = required_arg(install_matches, "type")?;
            let agent_type = parse_agent_type(agent_type_str)?;

            let result = agent_handlers::install_agent(agent_manager, vm_name, agent_type).await;
            if result.success {
                Ok(vec![result.message])
            } else {
                Err(anyhow::anyhow!(result.message))
            }
        }
        Some(("onboard", onboard_matches)) => {
            let vm_name = required_arg(onboard_matches, "vm")?;
            let agent_type_str = required_arg(onboard_matches, "type")?;
            let agent_type = parse_agent_type(agent_type_str)?;
            let provider = required_arg(onboard_matches, "provider")?.to_string();
            let model = onboard_matches
                .get_one::<String>("model")
                .map(String::to_string);
            let name = onboard_matches
                .get_one::<String>("name")
                .map(String::to_string);
            let api_key_name = onboard_matches
                .get_one::<String>("api-key-name")
                .map(String::to_string)
                .unwrap_or_else(|| "openrouter-api-key".to_string());

            let request = OnboardAgentRequest {
                name,
                agent_type,
                provider,
                model,
                api_key_name,
                capabilities: None,
                max_iterations: None,
                workspace_path: None,
            };

            let result = agent_handlers::onboard_agent(agent_manager, vm_name, request).await;
            if result.success {
                if let Some(instance) = result.data {
                    Ok(format_agent_instance(&instance))
                } else {
                    Ok(vec![result.message])
                }
            } else {
                Err(anyhow::anyhow!(result.message))
            }
        }
        Some(("list", list_matches)) => {
            let vm_name = required_arg(list_matches, "vm")?;
            let result = agent_handlers::list_agents(agent_manager, vm_name).await;
            if result.success {
                if let Some(agents) = result.data {
                    if agents.is_empty() {
                        Ok(vec![format!("No agents found in VM '{}'", vm_name)])
                    } else {
                        Ok(agents
                            .into_iter()
                            .map(|agent| format_agent_summary(&agent))
                            .collect())
                    }
                } else {
                    Ok(vec![result.message])
                }
            } else {
                Err(anyhow::anyhow!(result.message))
            }
        }
        Some(("get", get_matches)) => {
            let vm_name = required_arg(get_matches, "vm")?;
            let agent_id = required_arg(get_matches, "agent-id")?;
            let result = agent_handlers::get_agent(agent_manager, vm_name, agent_id).await;
            if result.success {
                if let Some(instance) = result.data {
                    Ok(format_agent_instance(&instance))
                } else {
                    Ok(vec![result.message])
                }
            } else {
                Err(anyhow::anyhow!(result.message))
            }
        }
        Some(("stop", stop_matches)) => {
            let vm_name = required_arg(stop_matches, "vm")?;
            let agent_id = required_arg(stop_matches, "agent-id")?;
            let result = agent_handlers::stop_agent(agent_manager, vm_name, agent_id).await;
            if result.success {
                Ok(vec![result.message])
            } else {
                Err(anyhow::anyhow!(result.message))
            }
        }
        Some(("delete", delete_matches)) => {
            let vm_name = required_arg(delete_matches, "vm")?;
            let agent_id = required_arg(delete_matches, "agent-id")?;
            let result = agent_handlers::delete_agent(agent_manager, vm_name, agent_id).await;
            if result.success {
                Ok(vec![result.message])
            } else {
                Err(anyhow::anyhow!(result.message))
            }
        }
        Some(("check", check_matches)) => {
            let vm_name = required_arg(check_matches, "vm")?;
            let agent_type_str = required_arg(check_matches, "type")?;
            let agent_type = parse_agent_type(agent_type_str)?;
            let result =
                agent_handlers::check_agent_installed(agent_manager, vm_name, agent_type).await;
            if result.success {
                Ok(vec![result.message])
            } else {
                Err(anyhow::anyhow!(result.message))
            }
        }
        _ => Ok(Vec::new()),
    }
}

fn parse_agent_type(s: &str) -> Result<AgentType> {
    match s {
        "picoclaw" => Ok(AgentType::Picoclaw),
        _ => bail!("unsupported agent type: {}", s),
    }
}

fn format_agent_summary(agent: &AgentInstance) -> String {
    let name = agent.name.as_deref().unwrap_or(&agent.id);
    format!(
        "{} | {:?} | {:?} | {}",
        name, agent.status, agent.config.agent_type, agent.config.provider_config.provider
    )
}

fn format_agent_instance(agent: &AgentInstance) -> Vec<String> {
    let mut lines = vec![
        format!("ID:       {}", agent.id),
        format!("Name:     {}", agent.name.as_deref().unwrap_or("<unnamed>")),
        format!("VM:       {}", agent.vm_name),
        format!("Type:     {:?}", agent.config.agent_type),
        format!("Status:   {:?}", agent.status),
        format!("Provider: {}", agent.config.provider_config.provider),
    ];

    if let Some(ref model) = agent.config.provider_config.model {
        lines.push(format!("Model:    {}", model));
    }

    if let Some(pid) = agent.pid {
        lines.push(format!("PID:      {}", pid));
    }

    lines.push(format!("Created:  {}", agent.created_at));

    if let Some(ref last_activity) = agent.last_activity {
        lines.push(format!("Active:   {}", last_activity));
    }

    if let Some(ref error) = agent.error_message {
        lines.push(format!("Error:    {}", error));
    }

    lines
}

fn required_arg<'a>(matches: &'a ArgMatches, name: &str) -> Result<&'a str> {
    matches
        .get_one::<String>(name)
        .map(String::as_str)
        .with_context(|| format!("missing required argument: {name}"))
}

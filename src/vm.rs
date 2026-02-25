use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::process::Command;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpawnVmRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VmStatusResponse {
    pub name: String,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv4: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_release: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_count: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_used: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_used: Option<u64>,
}

impl VmStatusResponse {
    pub fn minimal(name: impl Into<String>, state: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            state: state.into(),
            ipv4: None,
            release: None,
            image_release: None,
            cpu_count: None,
            memory_total: None,
            memory_used: None,
            disk_total: None,
            disk_used: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VmSummary {
    pub name: String,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv4: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release: Option<String>,
}

impl VmSummary {
    pub fn minimal(name: impl Into<String>, state: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            state: state.into(),
            ipv4: None,
            release: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum VmError {
    #[error("VM operation not implemented")]
    NotImplemented,
    #[error("failed to execute command: {0}")]
    CommandIo(String),
    #[error("multipass {action} failed with status {status_code}: {stderr}")]
    CommandFailed {
        action: &'static str,
        status_code: i32,
        stderr: String,
    },
    #[error("invalid multipass output for {action}: {reason}")]
    InvalidOutput {
        action: &'static str,
        reason: String,
    },
}

#[async_trait]
pub trait Multipass: Send + Sync {
    async fn launch(&self, name: &str) -> Result<(), VmError>;
    async fn start(&self, name: &str) -> Result<(), VmError>;
    async fn stop(&self, name: &str) -> Result<(), VmError>;
    async fn restart(&self, name: &str) -> Result<(), VmError>;
    async fn delete(&self, name: &str) -> Result<(), VmError>;
    async fn info(&self, name: &str) -> Result<VmStatusResponse, VmError>;
    async fn list(&self) -> Result<Vec<VmSummary>, VmError>;
}

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub status_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CommandOutput {
    pub fn success(stdout: impl Into<String>) -> Self {
        Self {
            status_code: 0,
            stdout: stdout.into(),
            stderr: String::new(),
        }
    }
}

#[async_trait]
pub trait CommandExecutor: Send + Sync {
    async fn run(&self, program: &str, args: &[String]) -> anyhow::Result<CommandOutput>;
}

#[derive(Debug, Clone, Default)]
pub struct TokioCommandExecutor;

#[async_trait]
impl CommandExecutor for TokioCommandExecutor {
    async fn run(&self, program: &str, args: &[String]) -> anyhow::Result<CommandOutput> {
        let output = Command::new(program).args(args).output().await?;
        Ok(CommandOutput {
            status_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct MultipassCli<E>
where
    E: CommandExecutor,
{
    executor: E,
}

impl<E> MultipassCli<E>
where
    E: CommandExecutor,
{
    pub fn new(executor: E) -> Self {
        Self { executor }
    }

    async fn run_command(
        &self,
        action: &'static str,
        args: Vec<String>,
    ) -> Result<CommandOutput, VmError> {
        let command_preview = format!("multipass {}", args.join(" "));
        info!(action = action, command = %command_preview, "running multipass command");

        let output = self
            .executor
            .run("multipass", &args)
            .await
            .map_err(|err| VmError::CommandIo(err.to_string()))?;

        if output.status_code != 0 {
            let trimmed_stdout = output.stdout.trim();
            if !trimmed_stdout.is_empty() {
                debug!(action = action, stdout = %trimmed_stdout, "multipass stdout");
            }
            let trimmed_stderr = output.stderr.trim();
            if !trimmed_stderr.is_empty() {
                warn!(action = action, stderr = %trimmed_stderr, "multipass stderr");
            }
            return Err(VmError::CommandFailed {
                action,
                status_code: output.status_code,
                stderr: output.stderr.trim().to_owned(),
            });
        }

        let trimmed_stderr = output.stderr.trim();
        if !trimmed_stderr.is_empty() {
            debug!(action = action, stderr = %trimmed_stderr, "multipass stderr");
        }
        info!(action = action, "multipass command completed");

        Ok(output)
    }

    fn parse_status_output(&self, name: &str, output: &str) -> Result<VmStatusResponse, VmError> {
        let value: Value = serde_json::from_str(output).map_err(|err| VmError::InvalidOutput {
            action: "status",
            reason: err.to_string(),
        })?;

        let info = value
            .get("info")
            .and_then(Value::as_object)
            .ok_or_else(|| VmError::InvalidOutput {
                action: "status",
                reason: "missing info object".to_owned(),
            })?;

        let vm = info.get(name).ok_or_else(|| VmError::InvalidOutput {
            action: "status",
            reason: format!("missing VM entry for {name}"),
        })?;

        let state = vm
            .get("state")
            .and_then(Value::as_str)
            .ok_or_else(|| VmError::InvalidOutput {
                action: "status",
                reason: "missing VM state".to_owned(),
            })?;

        // Extract optional fields
        let ipv4 = vm
            .get("ipv4")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            });

        let release = vm.get("release").and_then(Value::as_str).map(String::from);
        let image_release = vm.get("image_release").and_then(Value::as_str).map(String::from);
        let cpu_count = vm.get("cpu_count").and_then(Value::as_str).map(String::from);

        let memory_total = vm
            .get("memory")
            .and_then(|m| m.get("total"))
            .and_then(Value::as_u64);
        let memory_used = vm
            .get("memory")
            .and_then(|m| m.get("used"))
            .and_then(Value::as_u64);

        // Get first disk stats (usually sda1)
        let (disk_total, disk_used) = vm
            .get("disks")
            .and_then(Value::as_object)
            .and_then(|disks| disks.values().next())
            .map(|disk| {
                let total = disk.get("total").and_then(|v| v.as_str()).and_then(|s| s.parse::<u64>().ok());
                let used = disk.get("used").and_then(|v| v.as_str()).and_then(|s| s.parse::<u64>().ok());
                (total, used)
            })
            .unwrap_or((None, None));

        Ok(VmStatusResponse {
            name: name.to_owned(),
            state: state.to_owned(),
            ipv4,
            release,
            image_release,
            cpu_count,
            memory_total,
            memory_used,
            disk_total,
            disk_used,
        })
    }

    fn parse_list_output(&self, output: &str) -> Result<Vec<VmSummary>, VmError> {
        let value: Value = serde_json::from_str(output).map_err(|err| VmError::InvalidOutput {
            action: "list",
            reason: err.to_string(),
        })?;

        let list = value
            .get("list")
            .and_then(Value::as_array)
            .ok_or_else(|| VmError::InvalidOutput {
                action: "list",
                reason: "missing list array".to_owned(),
            })?;

        let mut vms = Vec::with_capacity(list.len());
        for item in list {
            let name = item
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| VmError::InvalidOutput {
                    action: "list",
                    reason: "missing VM name".to_owned(),
                })?;
            let state = item
                .get("state")
                .and_then(Value::as_str)
                .ok_or_else(|| VmError::InvalidOutput {
                    action: "list",
                    reason: "missing VM state".to_owned(),
                })?;

            let ipv4 = item
                .get("ipv4")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .map(String::from)
                        .collect()
                });

            let release = item.get("release").and_then(Value::as_str).map(String::from);

            vms.push(VmSummary {
                name: name.to_owned(),
                state: state.to_owned(),
                ipv4,
                release,
            });
        }

        Ok(vms)
    }
}

#[async_trait]
impl<E> Multipass for MultipassCli<E>
where
    E: CommandExecutor,
{
    async fn launch(&self, name: &str) -> Result<(), VmError> {
        self.run_command(
            "launch",
            vec!["launch".to_owned(), "--name".to_owned(), name.to_owned()],
        )
        .await?;
        Ok(())
    }

    async fn start(&self, name: &str) -> Result<(), VmError> {
        self.run_command("start", vec!["start".to_owned(), name.to_owned()])
            .await?;
        Ok(())
    }

    async fn stop(&self, name: &str) -> Result<(), VmError> {
        self.run_command("stop", vec!["stop".to_owned(), name.to_owned()])
            .await?;
        Ok(())
    }

    async fn restart(&self, name: &str) -> Result<(), VmError> {
        self.run_command("restart", vec!["restart".to_owned(), name.to_owned()])
            .await?;
        Ok(())
    }

    async fn delete(&self, name: &str) -> Result<(), VmError> {
        self.run_command("delete", vec!["delete".to_owned(), name.to_owned(), "--purge".to_owned()])
            .await?;
        Ok(())
    }

    async fn info(&self, name: &str) -> Result<VmStatusResponse, VmError> {
        let output = self
            .run_command(
                "info",
                vec![
                    "info".to_owned(),
                    name.to_owned(),
                    "--format".to_owned(),
                    "json".to_owned(),
                ],
            )
            .await?;

        self.parse_status_output(name, &output.stdout)
    }

    async fn list(&self) -> Result<Vec<VmSummary>, VmError> {
        let output = self
            .run_command("list", vec!["list".to_owned(), "--format".to_owned(), "json".to_owned()])
            .await?;
        self.parse_list_output(&output.stdout)
    }
}

#[derive(Clone)]
struct VmApiState {
    multipass: Arc<dyn Multipass>,
}

pub fn app(multipass: Arc<dyn Multipass>) -> Router {
    Router::new()
        .route("/v1/vm", post(spawn_vm).get(list_vms))
        .route("/v1/vm/", post(spawn_vm).get(list_vms))
        .route("/v1/vm/{name}", get(get_vm_status).delete(terminate_vm))
        .route("/v1/vm/{name}/", get(get_vm_status).delete(terminate_vm))
        .with_state(VmApiState { multipass })
}

async fn spawn_vm(
    State(state): State<VmApiState>,
    Json(request): Json<SpawnVmRequest>,
) -> Result<StatusCode, StatusCode> {
    state
        .multipass
        .launch(&request.name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::CREATED)
}

async fn list_vms(State(state): State<VmApiState>) -> Result<Json<Vec<VmSummary>>, StatusCode> {
    let vms = state
        .multipass
        .list()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(vms))
}

async fn get_vm_status(
    State(state): State<VmApiState>,
    Path(name): Path<String>,
) -> Result<Json<VmStatusResponse>, StatusCode> {
    let status = state
        .multipass
        .info(&name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(status))
}

async fn terminate_vm(
    State(state): State<VmApiState>,
    Path(name): Path<String>,
) -> Result<StatusCode, StatusCode> {
    state
        .multipass
        .stop(&name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

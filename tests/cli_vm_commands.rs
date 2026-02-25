use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use safepaw::{
    cli::{VmApi, build_cli, run_vm_subcommand},
    vm::{VmStatusResponse, VmSummary},
};

#[derive(Default)]
struct FakeState {
    calls: Vec<String>,
}

#[derive(Clone)]
struct FakeVmApi {
    state: Arc<Mutex<FakeState>>,
}

impl Default for FakeVmApi {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(FakeState::default())),
        }
    }
}

impl FakeVmApi {
    fn calls(&self) -> Vec<String> {
        self.state.lock().expect("poisoned fake state").calls.clone()
    }
}

#[async_trait]
impl VmApi for FakeVmApi {
    async fn launch(&self, name: &str) -> anyhow::Result<()> {
        self.state
            .lock()
            .expect("poisoned fake state")
            .calls
            .push(format!("launch:{name}"));
        Ok(())
    }

    async fn start(&self, name: &str) -> anyhow::Result<()> {
        self.state
            .lock()
            .expect("poisoned fake state")
            .calls
            .push(format!("start:{name}"));
        Ok(())
    }

    async fn stop(&self, name: &str) -> anyhow::Result<()> {
        self.state
            .lock()
            .expect("poisoned fake state")
            .calls
            .push(format!("stop:{name}"));
        Ok(())
    }

    async fn restart(&self, name: &str) -> anyhow::Result<()> {
        self.state
            .lock()
            .expect("poisoned fake state")
            .calls
            .push(format!("restart:{name}"));
        Ok(())
    }

    async fn delete(&self, name: &str) -> anyhow::Result<()> {
        self.state
            .lock()
            .expect("poisoned fake state")
            .calls
            .push(format!("delete:{name}"));
        Ok(())
    }

    async fn info(&self, name: &str) -> anyhow::Result<VmStatusResponse> {
        self.state
            .lock()
            .expect("poisoned fake state")
            .calls
            .push(format!("info:{name}"));
        Ok(VmStatusResponse::minimal(name, "Running"))
    }

    async fn list(&self) -> anyhow::Result<Vec<VmSummary>> {
        self.state
            .lock()
            .expect("poisoned fake state")
            .calls
            .push("list".to_owned());
        Ok(vec![
            VmSummary::minimal("agent-1", "Running"),
            VmSummary::minimal("agent-2", "Stopped"),
        ])
    }
}

#[tokio::test]
async fn vm_launch_command_produces_expected_output_and_call() {
    let api = FakeVmApi::default();
    let matches = build_cli()
        .try_get_matches_from(["safeclaw", "vm", "launch", "agent-1"])
        .expect("failed to parse CLI args");

    let lines = run_vm_subcommand(
        matches
            .subcommand_matches("vm")
            .expect("missing vm subcommand"),
        &api,
    )
    .await
    .expect("launch command failed");

    assert_eq!(lines, vec!["launched agent-1"]);
    assert_eq!(api.calls(), vec!["launch:agent-1"]);
}

#[tokio::test]
async fn vm_info_command_produces_expected_output_and_call() {
    let api = FakeVmApi::default();
    let matches = build_cli()
        .try_get_matches_from(["safeclaw", "vm", "info", "agent-1"])
        .expect("failed to parse CLI args");

    let lines = run_vm_subcommand(
        matches
            .subcommand_matches("vm")
            .expect("missing vm subcommand"),
        &api,
    )
    .await
    .expect("info command failed");

    assert_eq!(lines, vec!["Name:  agent-1", "State: Running"]);
    assert_eq!(api.calls(), vec!["info:agent-1"]);
}

#[tokio::test]
async fn vm_list_command_produces_expected_output_and_call() {
    let api = FakeVmApi::default();
    let matches = build_cli()
        .try_get_matches_from(["safeclaw", "vm", "list"])
        .expect("failed to parse CLI args");

    let lines = run_vm_subcommand(
        matches
            .subcommand_matches("vm")
            .expect("missing vm subcommand"),
        &api,
    )
    .await
    .expect("list command failed");

    assert_eq!(lines, vec!["agent-1 | Running", "agent-2 | Stopped"]);
    assert_eq!(api.calls(), vec!["list"]);
}

#[tokio::test]
async fn vm_stop_command_produces_expected_output_and_call() {
    let api = FakeVmApi::default();
    let matches = build_cli()
        .try_get_matches_from(["safeclaw", "vm", "stop", "agent-1"])
        .expect("failed to parse CLI args");

    let lines = run_vm_subcommand(
        matches
            .subcommand_matches("vm")
            .expect("missing vm subcommand"),
        &api,
    )
    .await
    .expect("stop command failed");

    assert_eq!(lines, vec!["stopped agent-1"]);
    assert_eq!(api.calls(), vec!["stop:agent-1"]);
}

mod common;

use common::FakeVmApi;
use safepaw::cli::{build_cli, run_vm_subcommand};
use safepaw::vm::VmSummary;

#[tokio::test]
async fn vm_launch_command_produces_expected_output_and_call() {
    let api = FakeVmApi::default().with_list_response(vec![
        VmSummary::minimal("agent-1", "Running"),
        VmSummary::minimal("agent-2", "Stopped"),
    ]);
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

    assert_eq!(lines, vec!["VM 'agent-1' launched successfully"]);
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
    let api = FakeVmApi::default().with_list_response(vec![
        VmSummary::minimal("agent-1", "Running"),
        VmSummary::minimal("agent-2", "Stopped"),
    ]);
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

    assert_eq!(lines, vec!["VM 'agent-1' stopped successfully"]);
    assert_eq!(api.calls(), vec!["stop:agent-1"]);
}

mod common;

use common::multipass_cli_with_outputs;
use safepaw::vm::{CommandOutput, LocalVmApi, Multipass, VmApi};
use std::sync::Arc;

// ============================================================================
// Multipass trait tests for exec and transfer
// ============================================================================

#[tokio::test]
async fn exec_sends_correct_multipass_command() {
    let (multipass, fake) = multipass_cli_with_outputs(vec![CommandOutput {
        status_code: 0,
        stdout: "/usr/bin/zeroclaw\n".to_owned(),
        stderr: String::new(),
    }]);

    let output = multipass
        .exec("test-vm", &["which".to_string(), "zeroclaw".to_string()])
        .await
        .expect("exec should work");

    assert_eq!(output.status_code, 0);
    assert_eq!(output.stdout, "/usr/bin/zeroclaw\n");

    assert_eq!(
        fake.calls(),
        vec![vec![
            "multipass".to_owned(),
            "exec".to_owned(),
            "test-vm".to_owned(),
            "--".to_owned(),
            "which".to_owned(),
            "zeroclaw".to_owned()
        ]]
    );
}

#[tokio::test]
async fn exec_returns_non_zero_exit_code_when_command_fails() {
    let (multipass, _fake) = multipass_cli_with_outputs(vec![CommandOutput {
        status_code: 1,
        stdout: String::new(),
        stderr: "command not found\n".to_owned(),
    }]);

    let result = multipass
        .exec("test-vm", &["nonexistent-command".to_string()])
        .await;

    // exec should return error for non-zero exit codes
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("exec"));
}

#[tokio::test]
async fn exec_with_multiple_args_works() {
    let (multipass, fake) =
        multipass_cli_with_outputs(vec![CommandOutput::success("file created\n")]);

    multipass
        .exec(
            "test-vm",
            &[
                "bash".to_string(),
                "-c".to_string(),
                "echo hello > /tmp/test.txt".to_string(),
            ],
        )
        .await
        .expect("exec should work");

    let calls = fake.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0][0], "multipass");
    assert_eq!(calls[0][1], "exec");
    assert_eq!(calls[0][2], "test-vm");
    assert_eq!(calls[0][3], "--");
    assert_eq!(calls[0][4], "bash");
    assert_eq!(calls[0][5], "-c");
    assert_eq!(calls[0][6], "echo hello > /tmp/test.txt");
}

#[tokio::test]
async fn transfer_sends_correct_multipass_command() {
    let (multipass, fake) = multipass_cli_with_outputs(vec![CommandOutput::success("")]);

    multipass
        .transfer("test-vm", "/local/path/script.sh", "/tmp/script.sh")
        .await
        .expect("transfer should work");

    assert_eq!(
        fake.calls(),
        vec![vec![
            "multipass".to_owned(),
            "transfer".to_owned(),
            "/local/path/script.sh".to_owned(),
            "test-vm:/tmp/script.sh".to_owned()
        ]]
    );
}

#[tokio::test]
async fn transfer_returns_error_when_file_not_found() {
    let (multipass, _fake) = multipass_cli_with_outputs(vec![CommandOutput {
        status_code: 1,
        stdout: String::new(),
        stderr: "file not found: /nonexistent/file.txt".to_owned(),
    }]);

    let result = multipass
        .transfer("test-vm", "/nonexistent/file.txt", "/tmp/file.txt")
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("transfer"));
    assert!(err.to_string().contains("file not found"));
}

// ============================================================================
// VmApi (LocalVmApi) tests for exec and transfer
// ============================================================================

#[tokio::test]
async fn vm_api_exec_delegates_to_multipass() {
    let (multipass_cli, fake) = multipass_cli_with_outputs(vec![CommandOutput {
        status_code: 0,
        stdout: "Hello from VM\n".to_owned(),
        stderr: String::new(),
    }]);
    let multipass = Arc::new(multipass_cli) as Arc<dyn Multipass>;
    let vm_api = LocalVmApi::new(multipass);

    let output = vm_api
        .exec(
            "test-vm",
            &["echo".to_string(), "Hello from VM".to_string()],
        )
        .await
        .expect("exec should work");

    assert_eq!(output.status_code, 0);
    assert_eq!(output.stdout, "Hello from VM\n");

    assert_eq!(
        fake.calls(),
        vec![vec![
            "multipass".to_owned(),
            "exec".to_owned(),
            "test-vm".to_owned(),
            "--".to_owned(),
            "echo".to_owned(),
            "Hello from VM".to_owned()
        ]]
    );
}

#[tokio::test]
async fn vm_api_transfer_delegates_to_multipass() {
    let (multipass_cli, fake) = multipass_cli_with_outputs(vec![CommandOutput::success("")]);
    let multipass = Arc::new(multipass_cli) as Arc<dyn Multipass>;
    let vm_api = LocalVmApi::new(multipass);

    vm_api
        .transfer("test-vm", "/local/file.txt", "/remote/file.txt")
        .await
        .expect("transfer should work");

    assert_eq!(
        fake.calls(),
        vec![vec![
            "multipass".to_owned(),
            "transfer".to_owned(),
            "/local/file.txt".to_owned(),
            "test-vm:/remote/file.txt".to_owned()
        ]]
    );
}

#[tokio::test]
async fn vm_api_exec_returns_error_on_failure() {
    let (multipass_cli, _fake) = multipass_cli_with_outputs(vec![CommandOutput {
        status_code: 127,
        stdout: String::new(),
        stderr: "command not found".to_owned(),
    }]);
    let multipass = Arc::new(multipass_cli) as Arc<dyn Multipass>;
    let vm_api = LocalVmApi::new(multipass);

    let result = vm_api.exec("test-vm", &["invalid-cmd".to_string()]).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("failed to exec command"));
}

#[tokio::test]
async fn vm_api_transfer_returns_error_on_failure() {
    let (multipass_cli, _fake) = multipass_cli_with_outputs(vec![CommandOutput {
        status_code: 1,
        stdout: String::new(),
        stderr: "permission denied".to_owned(),
    }]);
    let multipass = Arc::new(multipass_cli) as Arc<dyn Multipass>;
    let vm_api = LocalVmApi::new(multipass);

    let result = vm_api
        .transfer("test-vm", "/local/file.txt", "/root/file.txt")
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("failed to transfer file"));
}

// ============================================================================
// Integration-style tests with sequences
// ============================================================================

#[tokio::test]
async fn full_script_installation_sequence() {
    // Simulate: transfer script, chmod +x, execute
    let (multipass_cli, fake) = multipass_cli_with_outputs(vec![
        CommandOutput::success(""), // transfer
        CommandOutput::success(""), // chmod
        CommandOutput {
            // execute script
            status_code: 0,
            stdout: "Installation successful\n".to_owned(),
            stderr: String::new(),
        },
    ]);
    let multipass = Arc::new(multipass_cli) as Arc<dyn Multipass>;
    let vm_api = LocalVmApi::new(multipass);

    // Transfer script
    vm_api
        .transfer("test-vm", "/local/install.sh", "/tmp/install.sh")
        .await
        .expect("transfer should work");

    // Make executable
    vm_api
        .exec(
            "test-vm",
            &[
                "chmod".to_string(),
                "+x".to_string(),
                "/tmp/install.sh".to_string(),
            ],
        )
        .await
        .expect("chmod should work");

    // Execute
    let output = vm_api
        .exec(
            "test-vm",
            &["bash".to_string(), "/tmp/install.sh".to_string()],
        )
        .await
        .expect("script execution should work");

    assert_eq!(output.stdout, "Installation successful\n");

    let calls = fake.calls();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0][1], "transfer");
    assert_eq!(calls[1][1], "exec");
    assert_eq!(calls[1][4], "chmod");
    assert_eq!(calls[2][1], "exec");
    assert_eq!(calls[2][4], "bash");
}

#[tokio::test]
async fn check_if_command_exists_sequence() {
    // Simulate: which zeroclaw (not found)
    let (multipass_cli, fake) = multipass_cli_with_outputs(vec![CommandOutput {
        status_code: 1,
        stdout: String::new(),
        stderr: String::new(),
    }]);
    let multipass = Arc::new(multipass_cli) as Arc<dyn Multipass>;
    let vm_api = LocalVmApi::new(multipass);

    let result = vm_api
        .exec("test-vm", &["which".to_string(), "zeroclaw".to_string()])
        .await;

    // which returns non-zero when command not found
    assert!(result.is_err());

    let calls = fake.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0][4], "which");
    assert_eq!(calls[0][5], "zeroclaw");
}

#[tokio::test]
async fn check_if_command_exists_found() {
    // Simulate: which zeroclaw (found)
    let (multipass_cli, _fake) = multipass_cli_with_outputs(vec![CommandOutput {
        status_code: 0,
        stdout: "/usr/local/bin/zeroclaw\n".to_owned(),
        stderr: String::new(),
    }]);
    let multipass = Arc::new(multipass_cli) as Arc<dyn Multipass>;
    let vm_api = LocalVmApi::new(multipass);

    let output = vm_api
        .exec("test-vm", &["which".to_string(), "zeroclaw".to_string()])
        .await
        .expect("which should work when command exists");

    assert_eq!(output.status_code, 0);
    assert!(output.stdout.contains("/zeroclaw"));
}

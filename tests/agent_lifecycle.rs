mod common;

use std::sync::Arc;

use common::FakeVmApi;
use safepaw::agent::{
    AgentManager, AgentStatus, AgentType, LocalAgentManager, OnboardAgentRequest,
};
use safepaw::db::SafePawDb;
use safepaw::vm::CommandOutput;
use tempfile::TempDir;

fn setup_manager(
    fake_vm_api: FakeVmApi,
) -> (TempDir, Arc<SafePawDb>, Arc<FakeVmApi>, LocalAgentManager) {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = temp_dir.path().join("safepaw.data");
    let db = Arc::new(SafePawDb::open(&db_path).expect("DB should initialize"));
    let fake_vm_api = Arc::new(fake_vm_api);
    let agent_manager = LocalAgentManager::new_with_db(fake_vm_api.clone(), db.clone());

    (temp_dir, db, fake_vm_api, agent_manager)
}

fn onboard_request(name: Option<&str>) -> OnboardAgentRequest {
    OnboardAgentRequest {
        name: name.map(str::to_owned),
        agent_type: AgentType::Picoclaw,
        provider: "openrouter".to_owned(),
        model: Some("openrouter/auto".to_owned()),
        api_key_name: "openrouter-api-key".to_owned(),
        capabilities: Some(vec!["filesystem".to_owned(), "network".to_owned()]),
        max_iterations: Some(100),
        workspace_path: Some("/home/ubuntu/workspace".to_owned()),
    }
}

#[tokio::test]
async fn is_agent_installed_returns_true_when_picoclaw_found() {
    let (_temp_dir, _db, _fake_vm_api, agent_manager) =
        setup_manager(FakeVmApi::new().with_exec_response(Ok(CommandOutput {
            status_code: 0,
            stdout: "/usr/local/bin/picoclaw\n".to_owned(),
            stderr: String::new(),
        })));

    let installed = agent_manager
        .is_agent_installed("test-vm", &AgentType::Picoclaw)
        .await
        .expect("check should work");

    assert!(installed, "picoclaw should be detected as installed");
}

#[tokio::test]
async fn is_agent_installed_returns_false_when_picoclaw_not_found() {
    let (_temp_dir, _db, _fake_vm_api, agent_manager) =
        setup_manager(FakeVmApi::new().with_exec_response(Ok(CommandOutput {
            status_code: 1,
            stdout: String::new(),
            stderr: String::new(),
        })));

    let installed = agent_manager
        .is_agent_installed("test-vm", &AgentType::Picoclaw)
        .await
        .expect("check should work");

    assert!(!installed, "picoclaw should not be detected");
}

#[tokio::test]
async fn is_agent_installed_returns_false_on_exec_error() {
    let (_temp_dir, _db, _fake_vm_api, agent_manager) =
        setup_manager(FakeVmApi::new().with_exec_response(Err(anyhow::anyhow!("VM not running"))));

    let installed = agent_manager
        .is_agent_installed("test-vm", &AgentType::Picoclaw)
        .await
        .expect("check should work");

    assert!(!installed, "should return false on error");
}

#[tokio::test]
async fn install_agent_executes_deb_installer_with_checksum_verification() {
    let (_temp_dir, _db, fake_vm_api, agent_manager) = setup_manager(
        FakeVmApi::new().with_exec_response(Ok(CommandOutput::success(
            "==> picoclaw installation complete\n",
        ))),
    );

    agent_manager
        .install_agent("test-vm", &AgentType::Picoclaw)
        .await
        .expect("installation should succeed");

    let exec_calls = fake_vm_api.exec_calls();
    assert_eq!(
        exec_calls.len(),
        1,
        "install should execute one remote command"
    );
    assert_eq!(exec_calls[0].vm_name, "test-vm");
    assert_eq!(exec_calls[0].command[0], "bash");
    assert_eq!(exec_calls[0].command[1], "-lc");

    let script = &exec_calls[0].command[2];
    assert!(script.contains("uname -m"));
    assert!(script.contains("picoclaw_0.2.1_checksums.txt"));
    assert!(script.contains("picoclaw_x86_64.deb"));
    assert!(script.contains("picoclaw_aarch64.deb"));
    assert!(script.contains("sha256sum -c"));
    assert!(script.contains("dpkg -i"));
}

#[tokio::test]
async fn install_agent_fails_when_exec_fails() {
    let (_temp_dir, _db, _fake_vm_api, agent_manager) =
        setup_manager(FakeVmApi::new().with_exec_response(Err(anyhow::anyhow!("exec failed",))));

    let result = agent_manager
        .install_agent("test-vm", &AgentType::Picoclaw)
        .await;

    assert!(result.is_err(), "installation should fail");
    assert!(result.unwrap_err().to_string().contains("execute"));
}

#[tokio::test]
async fn install_agent_fails_when_script_execution_fails() {
    let (_temp_dir, _db, _fake_vm_api, agent_manager) =
        setup_manager(FakeVmApi::new().with_exec_response(Ok(CommandOutput {
            status_code: 1,
            stdout: String::new(),
            stderr: "Installation failed\n".to_owned(),
        })));

    let result = agent_manager
        .install_agent("test-vm", &AgentType::Picoclaw)
        .await;

    assert!(result.is_err(), "installation should fail");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Installation script failed")
    );
}

#[tokio::test]
async fn onboard_agent_fails_when_agent_not_installed() {
    let (_temp_dir, _db, _fake_vm_api, agent_manager) =
        setup_manager(FakeVmApi::new().with_exec_response(Ok(CommandOutput {
            status_code: 1,
            stdout: String::new(),
            stderr: String::new(),
        })));

    let result = agent_manager
        .onboard_agent("test-vm", onboard_request(Some("test-agent")))
        .await;

    assert!(
        result.is_err(),
        "onboard should fail when agent is not installed"
    );
    assert!(result.unwrap_err().to_string().contains("not installed"));
}

#[tokio::test]
async fn onboard_agent_creates_ready_instance_and_persists_it() {
    let (_temp_dir, db, fake_vm_api, agent_manager) = setup_manager(
        FakeVmApi::new()
            .with_exec_response(Ok(CommandOutput {
                status_code: 0,
                stdout: "/usr/local/bin/picoclaw\n".to_owned(),
                stderr: String::new(),
            }))
            .with_exec_response(Ok(CommandOutput::success(
                "==> picoclaw onboarding complete\n",
            ))),
    );

    let instance = agent_manager
        .onboard_agent("test-vm", onboard_request(Some("my-agent")))
        .await
        .expect("onboard should succeed");

    assert_eq!(instance.name, Some("my-agent".to_owned()));
    assert_eq!(instance.vm_name, "test-vm");
    assert_eq!(instance.status, AgentStatus::Ready);
    assert_eq!(instance.config.agent_type, AgentType::Picoclaw);
    assert_eq!(instance.config.provider_config.provider, "openrouter");
    assert_eq!(
        instance.config.provider_config.model,
        Some("openrouter/auto".to_owned())
    );
    assert_eq!(
        instance.config.capabilities,
        vec!["filesystem".to_owned(), "network".to_owned()]
    );
    assert_eq!(instance.config.max_iterations, Some(100));
    assert_eq!(
        instance.config.workspace_path,
        Some("/home/ubuntu/workspace".to_owned())
    );
    assert!(uuid::Uuid::parse_str(&instance.id).is_ok());

    let exec_calls = fake_vm_api.exec_calls();
    assert_eq!(
        exec_calls.len(),
        2,
        "onboard should check install then run script"
    );
    let onboard_script = &exec_calls[1].command[2];
    assert!(onboard_script.contains("openrouter"));
    assert!(onboard_script.contains("openrouter-api-key"));
    assert!(onboard_script.contains("openrouter/auto"));

    let reloaded_manager = LocalAgentManager::new_with_db(Arc::new(FakeVmApi::new()), db.clone());
    let persisted = reloaded_manager
        .get_agent("test-vm", &instance.id)
        .await
        .expect("persisted agent should load");

    assert_eq!(persisted.id, instance.id);
    assert_eq!(persisted.status, AgentStatus::Ready);
}

#[tokio::test]
async fn onboard_agent_without_optional_fields_uses_defaults() {
    let (_temp_dir, _db, _fake_vm_api, agent_manager) = setup_manager(
        FakeVmApi::new()
            .with_exec_response(Ok(CommandOutput {
                status_code: 0,
                stdout: "/usr/local/bin/picoclaw\n".to_owned(),
                stderr: String::new(),
            }))
            .with_exec_response(Ok(CommandOutput::success(
                "==> picoclaw onboarding complete\n",
            ))),
    );

    let instance = agent_manager
        .onboard_agent(
            "test-vm",
            OnboardAgentRequest {
                name: None,
                agent_type: AgentType::Picoclaw,
                provider: "anthropic".to_owned(),
                model: None,
                api_key_name: "anthropic-api-key".to_owned(),
                capabilities: None,
                max_iterations: None,
                workspace_path: None,
            },
        )
        .await
        .expect("onboard should succeed");

    assert_eq!(instance.name, None);
    assert_eq!(instance.status, AgentStatus::Ready);
    assert_eq!(instance.config.capabilities, Vec::<String>::new());
    assert_eq!(instance.config.max_iterations, None);
    assert_eq!(instance.config.workspace_path, None);
}

#[tokio::test]
async fn stop_agent_updates_status_in_store() {
    let (_temp_dir, db, _fake_vm_api, agent_manager) = setup_manager(
        FakeVmApi::new()
            .with_exec_response(Ok(CommandOutput {
                status_code: 0,
                stdout: "/usr/local/bin/picoclaw\n".to_owned(),
                stderr: String::new(),
            }))
            .with_exec_response(Ok(CommandOutput::success(
                "==> picoclaw onboarding complete\n",
            ))),
    );

    let instance = agent_manager
        .onboard_agent("test-vm", onboard_request(Some("test-agent")))
        .await
        .expect("onboard should succeed");

    agent_manager
        .stop_agent("test-vm", &instance.id)
        .await
        .expect("stop should succeed");

    let reloaded_manager = LocalAgentManager::new_with_db(Arc::new(FakeVmApi::new()), db.clone());
    let persisted = reloaded_manager
        .get_agent("test-vm", &instance.id)
        .await
        .expect("stopped agent should load");

    assert_eq!(persisted.status, AgentStatus::Stopped);
    assert!(persisted.last_activity.is_some());
}

#[tokio::test]
async fn delete_agent_removes_from_store() {
    let (_temp_dir, db, _fake_vm_api, agent_manager) = setup_manager(
        FakeVmApi::new()
            .with_exec_response(Ok(CommandOutput {
                status_code: 0,
                stdout: "/usr/local/bin/picoclaw\n".to_owned(),
                stderr: String::new(),
            }))
            .with_exec_response(Ok(CommandOutput::success(
                "==> picoclaw onboarding complete\n",
            ))),
    );

    let instance = agent_manager
        .onboard_agent("test-vm", onboard_request(Some("test-agent")))
        .await
        .expect("onboard should succeed");

    agent_manager
        .delete_agent("test-vm", &instance.id)
        .await
        .expect("delete should succeed");

    let reloaded_manager = LocalAgentManager::new_with_db(Arc::new(FakeVmApi::new()), db.clone());
    let result = reloaded_manager.get_agent("test-vm", &instance.id).await;

    assert!(result.is_err(), "agent should not exist after deletion");
}

#[tokio::test]
async fn agents_are_isolated_per_vm() {
    let (_temp_dir, _db, _fake_vm_api, agent_manager) = setup_manager(
        FakeVmApi::new()
            .with_exec_response(Ok(CommandOutput {
                status_code: 0,
                stdout: "/usr/local/bin/picoclaw\n".to_owned(),
                stderr: String::new(),
            }))
            .with_exec_response(Ok(CommandOutput::success(
                "==> picoclaw onboarding complete\n",
            )))
            .with_exec_response(Ok(CommandOutput {
                status_code: 0,
                stdout: "/usr/local/bin/picoclaw\n".to_owned(),
                stderr: String::new(),
            }))
            .with_exec_response(Ok(CommandOutput::success(
                "==> picoclaw onboarding complete\n",
            ))),
    );

    agent_manager
        .onboard_agent("vm-1", onboard_request(Some("agent-vm1")))
        .await
        .expect("first onboard should succeed");
    agent_manager
        .onboard_agent("vm-2", onboard_request(Some("agent-vm2")))
        .await
        .expect("second onboard should succeed");

    let vm1_agents = agent_manager
        .list_agents("vm-1")
        .await
        .expect("list should succeed");
    let vm2_agents = agent_manager
        .list_agents("vm-2")
        .await
        .expect("list should succeed");

    assert_eq!(vm1_agents.len(), 1);
    assert_eq!(vm2_agents.len(), 1);
    assert_eq!(vm1_agents[0].name, Some("agent-vm1".to_owned()));
    assert_eq!(vm2_agents[0].name, Some("agent-vm2".to_owned()));
}

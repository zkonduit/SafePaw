use std::{path::Path, sync::Arc};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{db::SafePawDb, vm::VmApi};

const AGENT_NAMESPACE: &str = "agents";
const INSTALLATION_NAMESPACE: &str = "agent_installations";
const PICOCLAW_VERSION: &str = "0.2.1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AgentType {
    Picoclaw,
}

impl AgentType {
    fn binary_name(&self) -> &'static str {
        match self {
            Self::Picoclaw => "picoclaw",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider: String,
    pub model: Option<String>,
    pub api_key_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent_type: AgentType,
    pub provider_config: ProviderConfig,
    pub capabilities: Vec<String>,
    pub max_iterations: Option<u32>,
    pub workspace_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Installing,
    Onboarding,
    Ready,
    Running,
    Stopped,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInstance {
    pub id: String,
    pub name: Option<String>,
    pub vm_name: String,
    pub config: AgentConfig,
    pub status: AgentStatus,
    pub pid: Option<u32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: Option<chrono::DateTime<chrono::Utc>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInstallation {
    pub vm_name: String,
    pub agent_type: AgentType,
    pub version: String,
    pub installed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentErrorDetails {
    pub code: String,
    pub operation: String,
    pub vm_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_type: Option<AgentType>,
    pub causes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct OnboardAgentRequest {
    pub name: Option<String>,
    pub agent_type: AgentType,
    pub provider: String,
    pub model: Option<String>,
    pub api_key_name: String,
    pub capabilities: Option<Vec<String>>,
    pub max_iterations: Option<u32>,
    pub workspace_path: Option<String>,
}

#[async_trait::async_trait]
pub trait AgentManager: Send + Sync {
    async fn onboard_agent(
        &self,
        vm_name: &str,
        request: OnboardAgentRequest,
    ) -> Result<AgentInstance>;
    async fn get_agent(&self, vm_name: &str, agent_id: &str) -> Result<AgentInstance>;
    async fn list_agents(&self, vm_name: &str) -> Result<Vec<AgentInstance>>;
    async fn stop_agent(&self, vm_name: &str, agent_id: &str) -> Result<()>;
    async fn delete_agent(&self, vm_name: &str, agent_id: &str) -> Result<()>;
    async fn is_agent_installed(&self, vm_name: &str, agent_type: &AgentType) -> Result<bool>;
    async fn install_agent(&self, vm_name: &str, agent_type: &AgentType) -> Result<()>;
}

pub struct LocalAgentManager {
    db: Arc<SafePawDb>,
    vm_api: Arc<dyn VmApi>,
}

impl LocalAgentManager {
    pub fn new(vm_api: Arc<dyn VmApi>) -> Result<Self> {
        let db = Arc::new(SafePawDb::open_default()?);
        Ok(Self::new_with_db(vm_api, db))
    }

    pub fn new_with_db(vm_api: Arc<dyn VmApi>, db: Arc<SafePawDb>) -> Self {
        Self { db, vm_api }
    }

    pub fn new_with_db_path(vm_api: Arc<dyn VmApi>, path: impl AsRef<Path>) -> Result<Self> {
        let db = Arc::new(SafePawDb::open(path)?);
        Ok(Self::new_with_db(vm_api, db))
    }

    fn generate_agent_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    fn agent_key(vm_name: &str, agent_id: &str) -> String {
        format!("{vm_name}:{agent_id}")
    }

    fn installation_key(vm_name: &str, agent_type: &AgentType) -> String {
        format!("{vm_name}:{agent_type:?}").to_lowercase()
    }

    fn install_script(agent_type: &AgentType) -> &'static str {
        match agent_type {
            AgentType::Picoclaw => include_str!("../scripts/vm-init/picoclaw/install-picoclaw.sh"),
        }
    }

    fn onboard_script(agent_type: &AgentType) -> &'static str {
        match agent_type {
            AgentType::Picoclaw => include_str!("../scripts/vm-init/picoclaw/onboard-picoclaw.sh"),
        }
    }

    fn save_agent(&self, agent: &AgentInstance) -> Result<()> {
        let key = Self::agent_key(&agent.vm_name, &agent.id);
        self.db.put_json(AGENT_NAMESPACE, &key, agent)
    }

    fn load_agent(&self, vm_name: &str, agent_id: &str) -> Result<Option<AgentInstance>> {
        let key = Self::agent_key(vm_name, agent_id);
        self.db.get_json(AGENT_NAMESPACE, &key)
    }

    fn load_agents(&self, vm_name: &str) -> Result<Vec<AgentInstance>> {
        let prefix = format!("{vm_name}:");
        let mut agents: Vec<AgentInstance> = self.db.list_json(AGENT_NAMESPACE, &prefix)?;
        agents.sort_by_key(|agent| agent.created_at);
        Ok(agents)
    }

    fn delete_agent_record(&self, vm_name: &str, agent_id: &str) -> Result<bool> {
        let key = Self::agent_key(vm_name, agent_id);
        self.db.delete(AGENT_NAMESPACE, &key)
    }

    fn save_installation(&self, vm_name: &str, agent_type: &AgentType) -> Result<()> {
        let record = AgentInstallation {
            vm_name: vm_name.to_owned(),
            agent_type: agent_type.clone(),
            version: PICOCLAW_VERSION.to_owned(),
            installed_at: chrono::Utc::now(),
        };

        self.db.put_json(
            INSTALLATION_NAMESPACE,
            &Self::installation_key(vm_name, agent_type),
            &record,
        )
    }

    fn render_onboard_command(
        &self,
        agent: &AgentInstance,
        request: &OnboardAgentRequest,
    ) -> Result<String> {
        let capabilities_json = serde_json::to_string(&agent.config.capabilities)
            .context("failed to serialize capabilities")?;
        let command = format!(
            "export SAFEPAW_AGENT_ID={agent_id} \
SAFEPAW_AGENT_NAME={agent_name} \
SAFEPAW_PROVIDER={provider} \
SAFEPAW_MODEL={model} \
SAFEPAW_API_KEY_NAME={api_key_name} \
SAFEPAW_WORKSPACE_PATH={workspace_path} \
SAFEPAW_MAX_ITERATIONS={max_iterations} \
SAFEPAW_CAPABILITIES_JSON={capabilities_json}\n{}",
            Self::onboard_script(&agent.config.agent_type),
            agent_id = shell_escape(&agent.id),
            agent_name = shell_escape(agent.name.as_deref().unwrap_or("")),
            provider = shell_escape(&request.provider),
            model = shell_escape(request.model.as_deref().unwrap_or("")),
            api_key_name = shell_escape(&request.api_key_name),
            workspace_path = shell_escape(agent.config.workspace_path.as_deref().unwrap_or("")),
            max_iterations = shell_escape(
                &agent
                    .config
                    .max_iterations
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ),
            capabilities_json = shell_escape(&capabilities_json),
        );

        Ok(command)
    }
}

#[async_trait::async_trait]
impl AgentManager for LocalAgentManager {
    async fn onboard_agent(
        &self,
        vm_name: &str,
        request: OnboardAgentRequest,
    ) -> Result<AgentInstance> {
        let _vm_info = self
            .vm_api
            .info(vm_name)
            .await
            .context("Failed to get VM info")?;

        let installed = self
            .is_agent_installed(vm_name, &request.agent_type)
            .await?;
        if !installed {
            return Err(anyhow::anyhow!(
                "Agent type {:?} is not installed in VM {}. Run installation first.",
                request.agent_type,
                vm_name
            ));
        }

        let agent = AgentInstance {
            id: Self::generate_agent_id(),
            name: request.name.clone(),
            vm_name: vm_name.to_owned(),
            config: AgentConfig {
                agent_type: request.agent_type.clone(),
                provider_config: ProviderConfig {
                    provider: request.provider.clone(),
                    model: request.model.clone(),
                    api_key_name: request.api_key_name.clone(),
                },
                capabilities: request.capabilities.clone().unwrap_or_default(),
                max_iterations: request.max_iterations,
                workspace_path: request.workspace_path.clone(),
            },
            status: AgentStatus::Onboarding,
            pid: None,
            created_at: chrono::Utc::now(),
            last_activity: None,
            error_message: None,
        };

        let command = self.render_onboard_command(&agent, &request)?;
        let output = self
            .vm_api
            .exec(vm_name, &["bash".to_owned(), "-lc".to_owned(), command])
            .await
            .context("Failed to execute onboarding script")?;

        if output.status_code != 0 {
            return Err(anyhow::anyhow!(
                "Onboarding script failed with status {}: {}",
                output.status_code,
                output.stderr
            ));
        }

        let mut ready_agent = agent;
        ready_agent.status = AgentStatus::Ready;
        self.save_agent(&ready_agent)?;

        Ok(ready_agent)
    }

    async fn get_agent(&self, vm_name: &str, agent_id: &str) -> Result<AgentInstance> {
        self.load_agent(vm_name, agent_id)?
            .ok_or_else(|| anyhow::anyhow!("Agent {} not found in VM {}", agent_id, vm_name))
    }

    async fn list_agents(&self, vm_name: &str) -> Result<Vec<AgentInstance>> {
        self.load_agents(vm_name)
    }

    async fn stop_agent(&self, vm_name: &str, agent_id: &str) -> Result<()> {
        let mut agent = self.get_agent(vm_name, agent_id).await?;
        agent.status = AgentStatus::Stopped;
        agent.last_activity = Some(chrono::Utc::now());
        self.save_agent(&agent)?;

        Ok(())
    }

    async fn delete_agent(&self, vm_name: &str, agent_id: &str) -> Result<()> {
        self.stop_agent(vm_name, agent_id).await?;
        let deleted = self.delete_agent_record(vm_name, agent_id)?;
        if !deleted {
            return Err(anyhow::anyhow!(
                "Agent {} not found in VM {}",
                agent_id,
                vm_name
            ));
        }

        Ok(())
    }

    async fn is_agent_installed(&self, vm_name: &str, agent_type: &AgentType) -> Result<bool> {
        let binary_name = agent_type.binary_name();
        let result = self
            .vm_api
            .exec(
                vm_name,
                &[
                    "bash".to_owned(),
                    "-lc".to_owned(),
                    format!("export PATH=\"$HOME/.local/bin:$PATH\" && which {binary_name}"),
                ],
            )
            .await;

        match result {
            Ok(output) => Ok(output.status_code == 0),
            Err(_) => Ok(false),
        }
    }

    async fn install_agent(&self, vm_name: &str, agent_type: &AgentType) -> Result<()> {
        let output = self
            .vm_api
            .exec(
                vm_name,
                &[
                    "bash".to_owned(),
                    "-lc".to_owned(),
                    Self::install_script(agent_type).to_owned(),
                ],
            )
            .await
            .context("Failed to execute installation script")?;

        if output.status_code != 0 {
            return Err(anyhow::anyhow!(
                "Installation script failed with status {}: {}",
                output.status_code,
                output.stderr
            ));
        }

        self.save_installation(vm_name, agent_type)?;

        Ok(())
    }
}

pub mod handlers {
    use super::*;
    use crate::util::HandlerResult;

    fn error_causes(error: &anyhow::Error) -> Vec<String> {
        error.chain().map(|cause| cause.to_string()).collect()
    }

    fn handler_error<T>(
        code: &str,
        operation: &str,
        vm_name: &str,
        agent_id: Option<&str>,
        agent_type: Option<&AgentType>,
        message: String,
        error: anyhow::Error,
    ) -> HandlerResult<T> {
        let details = serde_json::to_value(AgentErrorDetails {
            code: code.to_owned(),
            operation: operation.to_owned(),
            vm_name: vm_name.to_owned(),
            agent_id: agent_id.map(str::to_owned),
            agent_type: agent_type.cloned(),
            causes: error_causes(&error),
        })
        .expect("agent error details should serialize");

        HandlerResult::err_with_details(message, details)
    }

    pub async fn install_agent(
        agent_manager: &dyn AgentManager,
        vm_name: &str,
        agent_type: AgentType,
    ) -> HandlerResult<()> {
        match agent_manager.install_agent(vm_name, &agent_type).await {
            Ok(_) => HandlerResult::ok_with_message(format!(
                "Agent {:?} installed successfully in VM '{}'",
                agent_type, vm_name
            )),
            Err(e) => handler_error(
                "agent_install_failed",
                "install_agent",
                vm_name,
                None,
                Some(&agent_type),
                format!("Failed to install agent in VM '{}': {}", vm_name, e),
                e,
            ),
        }
    }

    pub async fn onboard_agent(
        agent_manager: &dyn AgentManager,
        vm_name: &str,
        request: OnboardAgentRequest,
    ) -> HandlerResult<AgentInstance> {
        let agent_type = request.agent_type.clone();

        match agent_manager.onboard_agent(vm_name, request).await {
            Ok(instance) => HandlerResult::ok(
                instance,
                format!("Agent onboarded successfully in VM '{}'", vm_name),
            ),
            Err(e) => handler_error(
                "agent_onboard_failed",
                "onboard_agent",
                vm_name,
                None,
                Some(&agent_type),
                format!("Failed to onboard agent in VM '{}': {}", vm_name, e),
                e,
            ),
        }
    }

    pub async fn get_agent(
        agent_manager: &dyn AgentManager,
        vm_name: &str,
        agent_id: &str,
    ) -> HandlerResult<AgentInstance> {
        match agent_manager.get_agent(vm_name, agent_id).await {
            Ok(instance) => HandlerResult::ok(instance, "Agent retrieved successfully"),
            Err(e) => handler_error(
                "agent_get_failed",
                "get_agent",
                vm_name,
                Some(agent_id),
                None,
                format!(
                    "Failed to get agent '{}' in VM '{}': {}",
                    agent_id, vm_name, e
                ),
                e,
            ),
        }
    }

    pub async fn list_agents(
        agent_manager: &dyn AgentManager,
        vm_name: &str,
    ) -> HandlerResult<Vec<AgentInstance>> {
        match agent_manager.list_agents(vm_name).await {
            Ok(agents) => {
                let count = agents.len();
                HandlerResult::ok(
                    agents,
                    format!("Found {} agent(s) in VM '{}'", count, vm_name),
                )
            }
            Err(e) => handler_error(
                "agent_list_failed",
                "list_agents",
                vm_name,
                None,
                None,
                format!("Failed to list agents in VM '{}': {}", vm_name, e),
                e,
            ),
        }
    }

    pub async fn stop_agent(
        agent_manager: &dyn AgentManager,
        vm_name: &str,
        agent_id: &str,
    ) -> HandlerResult<()> {
        match agent_manager.stop_agent(vm_name, agent_id).await {
            Ok(_) => {
                HandlerResult::ok_with_message(format!("Agent '{}' stopped successfully", agent_id))
            }
            Err(e) => handler_error(
                "agent_stop_failed",
                "stop_agent",
                vm_name,
                Some(agent_id),
                None,
                format!(
                    "Failed to stop agent '{}' in VM '{}': {}",
                    agent_id, vm_name, e
                ),
                e,
            ),
        }
    }

    pub async fn delete_agent(
        agent_manager: &dyn AgentManager,
        vm_name: &str,
        agent_id: &str,
    ) -> HandlerResult<()> {
        match agent_manager.delete_agent(vm_name, agent_id).await {
            Ok(_) => {
                HandlerResult::ok_with_message(format!("Agent '{}' deleted successfully", agent_id))
            }
            Err(e) => handler_error(
                "agent_delete_failed",
                "delete_agent",
                vm_name,
                Some(agent_id),
                None,
                format!(
                    "Failed to delete agent '{}' in VM '{}': {}",
                    agent_id, vm_name, e
                ),
                e,
            ),
        }
    }

    pub async fn check_agent_installed(
        agent_manager: &dyn AgentManager,
        vm_name: &str,
        agent_type: AgentType,
    ) -> HandlerResult<bool> {
        match agent_manager.is_agent_installed(vm_name, &agent_type).await {
            Ok(installed) => {
                let message = if installed {
                    format!("Agent {:?} is installed in VM '{}'", agent_type, vm_name)
                } else {
                    format!(
                        "Agent {:?} is not installed in VM '{}'",
                        agent_type, vm_name
                    )
                };
                HandlerResult::ok(installed, message)
            }
            Err(e) => handler_error(
                "agent_installation_check_failed",
                "check_agent_installed",
                vm_name,
                None,
                Some(&agent_type),
                format!(
                    "Failed to check agent installation for VM '{}': {}",
                    vm_name, e
                ),
                e,
            ),
        }
    }
}

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_id_generation() {
        let id1 = LocalAgentManager::generate_agent_id();
        let id2 = LocalAgentManager::generate_agent_id();

        assert_ne!(id1, id2);
        assert!(uuid::Uuid::parse_str(&id1).is_ok());
        assert!(uuid::Uuid::parse_str(&id2).is_ok());
    }
}

// Shared test utilities for SafePaw tests

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use safepaw::vm::{
    CommandExecutor, CommandOutput, Multipass, MultipassCli, VmApi, VmStatusResponse, VmSummary,
};

// ============================================================================
// FakeExecutor - Mock CommandExecutor for testing
// ============================================================================

#[derive(Clone)]
pub struct FakeExecutor {
    calls: Arc<Mutex<Vec<Vec<String>>>>,
    outputs: Arc<Mutex<VecDeque<CommandOutput>>>,
}

impl FakeExecutor {
    pub fn new(outputs: Vec<CommandOutput>) -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            outputs: Arc::new(Mutex::new(outputs.into())),
        }
    }

    pub fn calls(&self) -> Vec<Vec<String>> {
        self.calls.lock().expect("poisoned calls mutex").clone()
    }
}

#[async_trait]
impl CommandExecutor for FakeExecutor {
    async fn run(&self, program: &str, args: &[String]) -> anyhow::Result<CommandOutput> {
        let mut call = Vec::with_capacity(args.len() + 1);
        call.push(program.to_owned());
        call.extend(args.iter().cloned());

        self.calls.lock().expect("poisoned calls mutex").push(call);

        self.outputs
            .lock()
            .expect("poisoned outputs mutex")
            .pop_front()
            .ok_or_else(|| anyhow::anyhow!("no fake output available"))
    }
}

// ============================================================================
// FakeMultipass - Mock Multipass trait for testing
// ============================================================================

#[derive(Clone)]
pub struct FakeMultipass {
    calls: Arc<Mutex<Vec<String>>>,
    responses: Arc<Mutex<FakeMultipassResponses>>,
    default_statuses: Arc<Mutex<std::collections::HashMap<String, VmStatusResponse>>>,
    default_list: Vec<VmSummary>,
}

#[derive(Default)]
struct FakeMultipassResponses {
    launch: VecDeque<Result<(), safepaw::vm::VmError>>,
    start: VecDeque<Result<(), safepaw::vm::VmError>>,
    stop: VecDeque<Result<(), safepaw::vm::VmError>>,
    restart: VecDeque<Result<(), safepaw::vm::VmError>>,
    delete: VecDeque<Result<(), safepaw::vm::VmError>>,
    info: VecDeque<Result<VmStatusResponse, safepaw::vm::VmError>>,
    list: VecDeque<Result<Vec<VmSummary>, safepaw::vm::VmError>>,
    exec: VecDeque<Result<CommandOutput, safepaw::vm::VmError>>,
    transfer: VecDeque<Result<(), safepaw::vm::VmError>>,
}

impl Default for FakeMultipass {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeMultipass {
    pub fn new() -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            responses: Arc::new(Mutex::new(FakeMultipassResponses::default())),
            default_statuses: Arc::new(Mutex::new(std::collections::HashMap::new())),
            default_list: vec![],
        }
    }

    pub fn with_status(self, name: &str, state: &str) -> Self {
        self.default_statuses
            .lock()
            .unwrap()
            .insert(name.to_owned(), VmStatusResponse::minimal(name, state));
        self
    }

    pub fn with_list(mut self, list: Vec<VmSummary>) -> Self {
        self.default_list = list;
        self
    }

    pub fn with_launch_response(self, response: Result<(), safepaw::vm::VmError>) -> Self {
        self.responses.lock().unwrap().launch.push_back(response);
        self
    }

    pub fn with_info_response(
        self,
        response: Result<VmStatusResponse, safepaw::vm::VmError>,
    ) -> Self {
        self.responses.lock().unwrap().info.push_back(response);
        self
    }

    pub fn with_list_response(
        self,
        response: Result<Vec<VmSummary>, safepaw::vm::VmError>,
    ) -> Self {
        self.responses.lock().unwrap().list.push_back(response);
        self
    }

    pub fn with_exec_response(self, response: Result<CommandOutput, safepaw::vm::VmError>) -> Self {
        self.responses.lock().unwrap().exec.push_back(response);
        self
    }

    pub fn with_transfer_response(self, response: Result<(), safepaw::vm::VmError>) -> Self {
        self.responses.lock().unwrap().transfer.push_back(response);
        self
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    fn record_call(&self, call: String) {
        self.calls.lock().unwrap().push(call);
    }
}

#[async_trait]
impl Multipass for FakeMultipass {
    async fn launch(&self, name: &str) -> Result<(), safepaw::vm::VmError> {
        self.record_call(format!("launch:{}", name));
        self.responses
            .lock()
            .unwrap()
            .launch
            .pop_front()
            .unwrap_or(Ok(()))
    }

    async fn start(&self, name: &str) -> Result<(), safepaw::vm::VmError> {
        self.record_call(format!("start:{}", name));
        self.responses
            .lock()
            .unwrap()
            .start
            .pop_front()
            .unwrap_or(Ok(()))
    }

    async fn stop(&self, name: &str) -> Result<(), safepaw::vm::VmError> {
        self.record_call(format!("stop:{}", name));
        self.responses
            .lock()
            .unwrap()
            .stop
            .pop_front()
            .unwrap_or(Ok(()))
    }

    async fn restart(&self, name: &str) -> Result<(), safepaw::vm::VmError> {
        self.record_call(format!("restart:{}", name));
        self.responses
            .lock()
            .unwrap()
            .restart
            .pop_front()
            .unwrap_or(Ok(()))
    }

    async fn delete(&self, name: &str) -> Result<(), safepaw::vm::VmError> {
        self.record_call(format!("delete:{}", name));
        self.responses
            .lock()
            .unwrap()
            .delete
            .pop_front()
            .unwrap_or(Ok(()))
    }

    async fn info(&self, name: &str) -> Result<VmStatusResponse, safepaw::vm::VmError> {
        self.record_call(format!("info:{}", name));
        self.responses
            .lock()
            .unwrap()
            .info
            .pop_front()
            .unwrap_or_else(|| {
                Ok(self
                    .default_statuses
                    .lock()
                    .unwrap()
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| VmStatusResponse::minimal(name, "Running")))
            })
    }

    async fn list(&self) -> Result<Vec<VmSummary>, safepaw::vm::VmError> {
        self.record_call("list".to_owned());
        self.responses
            .lock()
            .unwrap()
            .list
            .pop_front()
            .unwrap_or_else(|| Ok(self.default_list.clone()))
    }

    async fn exec(
        &self,
        _name: &str,
        _command: &[String],
    ) -> Result<CommandOutput, safepaw::vm::VmError> {
        self.responses
            .lock()
            .unwrap()
            .exec
            .pop_front()
            .unwrap_or(Ok(CommandOutput::success("")))
    }

    async fn transfer(
        &self,
        _name: &str,
        _source: &str,
        _destination: &str,
    ) -> Result<(), safepaw::vm::VmError> {
        self.responses
            .lock()
            .unwrap()
            .transfer
            .pop_front()
            .unwrap_or(Ok(()))
    }
}

// ============================================================================
// FakeVmApi - Mock VmApi trait for testing
// ============================================================================

#[derive(Clone)]
pub struct FakeVmApi {
    calls: Arc<Mutex<Vec<String>>>,
    exec_calls: Arc<Mutex<Vec<ExecCall>>>,
    exec_responses: Arc<Mutex<VecDeque<anyhow::Result<CommandOutput>>>>,
    transfer_responses: Arc<Mutex<VecDeque<anyhow::Result<()>>>>,
    info_response: VmStatusResponse,
    list_response: Vec<VmSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecCall {
    pub vm_name: String,
    pub command: Vec<String>,
}

impl Default for FakeVmApi {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeVmApi {
    pub fn new() -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            exec_calls: Arc::new(Mutex::new(Vec::new())),
            exec_responses: Arc::new(Mutex::new(VecDeque::new())),
            transfer_responses: Arc::new(Mutex::new(VecDeque::new())),
            info_response: VmStatusResponse::minimal("test-vm", "Running"),
            list_response: vec![],
        }
    }

    pub fn with_exec_response(self, response: anyhow::Result<CommandOutput>) -> Self {
        self.exec_responses.lock().unwrap().push_back(response);
        self
    }

    pub fn with_transfer_response(self, response: anyhow::Result<()>) -> Self {
        self.transfer_responses.lock().unwrap().push_back(response);
        self
    }

    pub fn with_info_response(mut self, response: VmStatusResponse) -> Self {
        self.info_response = response;
        self
    }

    pub fn with_list_response(mut self, response: Vec<VmSummary>) -> Self {
        self.list_response = response;
        self
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    pub fn exec_calls(&self) -> Vec<ExecCall> {
        self.exec_calls.lock().unwrap().clone()
    }

    fn record_call(&self, call: String) {
        self.calls.lock().unwrap().push(call);
    }
}

#[async_trait]
impl VmApi for FakeVmApi {
    async fn launch(&self, name: &str) -> anyhow::Result<()> {
        self.record_call(format!("launch:{}", name));
        Ok(())
    }

    async fn start(&self, name: &str) -> anyhow::Result<()> {
        self.record_call(format!("start:{}", name));
        Ok(())
    }

    async fn stop(&self, name: &str) -> anyhow::Result<()> {
        self.record_call(format!("stop:{}", name));
        Ok(())
    }

    async fn restart(&self, name: &str) -> anyhow::Result<()> {
        self.record_call(format!("restart:{}", name));
        Ok(())
    }

    async fn delete(&self, name: &str) -> anyhow::Result<()> {
        self.record_call(format!("delete:{}", name));
        Ok(())
    }

    async fn info(&self, name: &str) -> anyhow::Result<VmStatusResponse> {
        self.record_call(format!("info:{}", name));
        // Return a response with the actual VM name instead of the default "test-vm"
        let mut response = self.info_response.clone();
        response.name = name.to_owned();
        Ok(response)
    }

    async fn list(&self) -> anyhow::Result<Vec<VmSummary>> {
        self.record_call("list".to_owned());
        Ok(self.list_response.clone())
    }

    async fn exec(&self, name: &str, command: &[String]) -> anyhow::Result<CommandOutput> {
        self.exec_calls.lock().unwrap().push(ExecCall {
            vm_name: name.to_owned(),
            command: command.to_vec(),
        });
        self.exec_responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Ok(CommandOutput::success("")))
    }

    async fn transfer(&self, _name: &str, _source: &str, _destination: &str) -> anyhow::Result<()> {
        self.transfer_responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Ok(()))
    }
}

// ============================================================================
// Helper functions
// ============================================================================

pub fn multipass_cli_with_outputs(
    outputs: Vec<CommandOutput>,
) -> (MultipassCli<FakeExecutor>, FakeExecutor) {
    let fake = FakeExecutor::new(outputs);
    let cli = MultipassCli::new(fake.clone());
    (cli, fake)
}

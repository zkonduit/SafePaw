use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use safepaw::vm::{LocalVmApi, Multipass, VmApi, VmError, VmStatusResponse, VmSummary};

#[derive(Default)]
struct FakeState {
    calls: Vec<String>,
    statuses: HashMap<String, VmStatusResponse>,
    listed_vms: Vec<VmSummary>,
}

#[derive(Clone, Default)]
struct FakeMultipass {
    state: Arc<Mutex<FakeState>>,
}

impl FakeMultipass {
    fn with_status(self, name: &str, state: &str) -> Self {
        self.state
            .lock()
            .expect("poisoned fake state")
            .statuses
            .insert(name.to_owned(), VmStatusResponse::minimal(name, state));
        self
    }

    fn with_list(self, listed_vms: Vec<VmSummary>) -> Self {
        self.state.lock().expect("poisoned fake state").listed_vms = listed_vms;
        self
    }

    fn calls(&self) -> Vec<String> {
        self.state
            .lock()
            .expect("poisoned fake state")
            .calls
            .clone()
    }
}

#[async_trait]
impl Multipass for FakeMultipass {
    async fn launch(&self, name: &str) -> Result<(), VmError> {
        self.state
            .lock()
            .expect("poisoned fake state")
            .calls
            .push(format!("launch:{name}"));
        Ok(())
    }

    async fn start(&self, name: &str) -> Result<(), VmError> {
        self.state
            .lock()
            .expect("poisoned fake state")
            .calls
            .push(format!("start:{name}"));
        Ok(())
    }

    async fn stop(&self, name: &str) -> Result<(), VmError> {
        self.state
            .lock()
            .expect("poisoned fake state")
            .calls
            .push(format!("stop:{name}"));
        Ok(())
    }

    async fn restart(&self, name: &str) -> Result<(), VmError> {
        self.state
            .lock()
            .expect("poisoned fake state")
            .calls
            .push(format!("restart:{name}"));
        Ok(())
    }

    async fn delete(&self, name: &str) -> Result<(), VmError> {
        self.state
            .lock()
            .expect("poisoned fake state")
            .calls
            .push(format!("delete:{name}"));
        Ok(())
    }

    async fn info(&self, name: &str) -> Result<VmStatusResponse, VmError> {
        let mut state = self.state.lock().expect("poisoned fake state");
        state.calls.push(format!("info:{name}"));
        Ok(state
            .statuses
            .get(name)
            .cloned()
            .unwrap_or_else(|| VmStatusResponse::minimal(name, "Unknown")))
    }

    async fn list(&self) -> Result<Vec<VmSummary>, VmError> {
        let mut state = self.state.lock().expect("poisoned fake state");
        state.calls.push("list".to_owned());
        Ok(state.listed_vms.clone())
    }
}

#[tokio::test]
async fn launch_calls_multipass() {
    let fake = FakeMultipass::default();
    let api = LocalVmApi::new(Arc::new(fake.clone()));

    api.launch("agent-1").await.expect("launch should succeed");

    assert_eq!(fake.calls(), vec!["launch:agent-1"]);
}

#[tokio::test]
async fn info_returns_vm_info() {
    let fake = FakeMultipass::default().with_status("agent-1", "Running");
    let api = LocalVmApi::new(Arc::new(fake.clone()));

    let info = api.info("agent-1").await.expect("info should succeed");

    assert_eq!(info.name, "agent-1");
    assert_eq!(info.state, "Running");
    assert_eq!(fake.calls(), vec!["info:agent-1"]);
}

#[tokio::test]
async fn list_returns_vms_from_multipass() {
    let fake = FakeMultipass::default().with_list(vec![
        VmSummary::minimal("agent-1", "Running"),
        VmSummary::minimal("agent-2", "Stopped"),
    ]);
    let api = LocalVmApi::new(Arc::new(fake.clone()));

    let listed = api.list().await.expect("list should succeed");

    assert_eq!(fake.calls(), vec!["list"]);
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].name, "agent-1");
    assert_eq!(listed[0].state, "Running");
    assert_eq!(listed[1].name, "agent-2");
    assert_eq!(listed[1].state, "Stopped");
}

#[tokio::test]
async fn stop_stops_vm() {
    let fake = FakeMultipass::default();
    let api = LocalVmApi::new(Arc::new(fake.clone()));

    api.stop("agent-1").await.expect("stop should succeed");

    assert_eq!(fake.calls(), vec!["stop:agent-1"]);
}

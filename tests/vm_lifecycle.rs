use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use safepaw::vm::{self, Multipass, VmError, VmStatusResponse, VmSummary};
use serde_json::{Value, json};
use tower::util::ServiceExt;

#[derive(Default)]
struct FakeState {
    calls: Vec<String>,
    status_by_name: HashMap<String, VmStatusResponse>,
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
            .status_by_name
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
            .status_by_name
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
async fn spawn_vm_returns_created_and_launches_vm() {
    let fake = FakeMultipass::default();
    let app = vm::app(Arc::new(fake.clone()));

    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/vm")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "agent-1"
            })
            .to_string(),
        ))
        .expect("failed to build request");

    let response = app.oneshot(request).await.expect("failed to call vm app");

    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(fake.calls(), vec!["launch:agent-1"]);
}

#[tokio::test]
async fn get_vm_status_returns_current_vm_state() {
    let fake = FakeMultipass::default().with_status("agent-1", "Running");
    let app = vm::app(Arc::new(fake.clone()));

    let request = Request::builder()
        .method(Method::GET)
        .uri("/v1/vm/agent-1/")
        .body(Body::empty())
        .expect("failed to build request");

    let response = app.oneshot(request).await.expect("failed to call vm app");

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("failed to read body");
    let json: Value = serde_json::from_slice(&body).expect("invalid JSON body");

    assert_eq!(json["name"], "agent-1");
    assert_eq!(json["state"], "Running");
    assert_eq!(fake.calls(), vec!["info:agent-1"]);
}

#[tokio::test]
async fn list_vms_returns_known_vms() {
    let fake = FakeMultipass::default().with_list(vec![
        VmSummary::minimal("agent-1", "Running"),
        VmSummary::minimal("agent-2", "Stopped"),
    ]);
    let app = vm::app(Arc::new(fake.clone()));

    let request = Request::builder()
        .method(Method::GET)
        .uri("/v1/vm/")
        .body(Body::empty())
        .expect("failed to build request");

    let response = app.oneshot(request).await.expect("failed to call vm app");

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("failed to read body");
    let json: Value = serde_json::from_slice(&body).expect("invalid JSON body");

    assert_eq!(json.as_array().expect("expected array").len(), 2);
    assert_eq!(json[0]["name"], "agent-1");
    assert_eq!(json[0]["state"], "Running");
    assert_eq!(json[1]["name"], "agent-2");
    assert_eq!(json[1]["state"], "Stopped");
    assert_eq!(fake.calls(), vec!["list"]);
}

#[tokio::test]
async fn terminate_vm_returns_no_content_and_stops_vm() {
    let fake = FakeMultipass::default();
    let app = vm::app(Arc::new(fake.clone()));

    let request = Request::builder()
        .method(Method::DELETE)
        .uri("/v1/vm/agent-1")
        .body(Body::empty())
        .expect("failed to build request");

    let response = app.oneshot(request).await.expect("failed to call vm app");

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(fake.calls(), vec!["stop:agent-1"]);
}

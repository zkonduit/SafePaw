use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use safepaw::{
    server::create_api_router,
    vm::{VmApi, VmStatusResponse, VmSummary},
};
use tower::ServiceExt;

#[derive(Default)]
struct FakeState {
    vms: Vec<VmSummary>,
}

#[derive(Clone, Default)]
struct FakeVmApi {
    state: Arc<Mutex<FakeState>>,
}

impl FakeVmApi {
    fn with_vms(self, vms: Vec<VmSummary>) -> Self {
        self.state.lock().expect("poisoned fake state").vms = vms;
        self
    }
}

#[async_trait]
impl VmApi for FakeVmApi {
    async fn launch(&self, _name: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn start(&self, _name: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn stop(&self, _name: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn restart(&self, _name: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn delete(&self, _name: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn info(&self, name: &str) -> anyhow::Result<VmStatusResponse> {
        Ok(VmStatusResponse {
            name: name.to_owned(),
            state: "Running".to_owned(),
            ipv4: Some(vec!["192.168.1.100".to_owned()]),
            release: Some("Ubuntu 22.04".to_owned()),
            image_release: Some("Ubuntu 22.04 LTS".to_owned()),
            cpu_count: Some("2".to_owned()),
            memory_total: Some(2 * 1024 * 1024 * 1024), // 2 GiB
            memory_used: Some(1024 * 1024 * 1024),      // 1 GiB
            disk_total: Some(10 * 1024 * 1024 * 1024),  // 10 GiB
            disk_used: Some(5 * 1024 * 1024 * 1024),    // 5 GiB
        })
    }

    async fn list(&self) -> anyhow::Result<Vec<VmSummary>> {
        Ok(self.state.lock().expect("poisoned fake state").vms.clone())
    }
}

#[tokio::test]
async fn health_check_returns_ok() {
    let fake_api = FakeVmApi::default();
    let app_state = safepaw::server::AppState::new(Arc::new(fake_api));
    let app = create_api_router(app_state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn list_vms_returns_empty_array_when_no_vms() {
    let fake_api = FakeVmApi::default();
    let app_state = safepaw::server::AppState::new(Arc::new(fake_api));
    let app = create_api_router(app_state);

    let response = app
        .oneshot(Request::builder().uri("/vms").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn list_vms_returns_vms() {
    let fake_api = FakeVmApi::default().with_vms(vec![
        VmSummary {
            name: "agent-1".to_owned(),
            state: "Running".to_owned(),
            ipv4: Some(vec!["192.168.1.100".to_owned()]),
            release: Some("Ubuntu 22.04".to_owned()),
        },
        VmSummary {
            name: "agent-2".to_owned(),
            state: "Stopped".to_owned(),
            ipv4: None,
            release: Some("Ubuntu 22.04".to_owned()),
        },
    ]);
    let app_state = safepaw::server::AppState::new(Arc::new(fake_api));
    let app = create_api_router(app_state);

    let response = app
        .oneshot(Request::builder().uri("/vms").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let vms: Vec<safepaw::server::VmStatusDto> = serde_json::from_slice(&body).unwrap();

    assert_eq!(vms.len(), 2);
    assert_eq!(vms[0].name, "agent-1");
    assert_eq!(vms[0].state, "Running");
    assert_eq!(vms[1].name, "agent-2");
    assert_eq!(vms[1].state, "Stopped");
}

#[tokio::test]
async fn get_vm_info_returns_vm_details() {
    let fake_api = FakeVmApi::default();
    let app_state = safepaw::server::AppState::new(Arc::new(fake_api));
    let app = create_api_router(app_state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/vms/agent-1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let vm: safepaw::server::VmStatusDto = serde_json::from_slice(&body).unwrap();

    assert_eq!(vm.name, "agent-1");
    assert_eq!(vm.state, "Running");
    assert_eq!(vm.memory_total, Some(2 * 1024 * 1024 * 1024));
    assert_eq!(vm.memory_used, Some(1024 * 1024 * 1024));
    assert_eq!(vm.disk_total, Some(10 * 1024 * 1024 * 1024));
    assert_eq!(vm.disk_used, Some(5 * 1024 * 1024 * 1024));
}

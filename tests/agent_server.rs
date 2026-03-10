mod common;

use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::FakeVmApi;
use safepaw::agent::LocalAgentManager;
use safepaw::db::SafePawDb;
use safepaw::server::{AppState, create_api_router};
use safepaw::vm::CommandOutput;
use serde_json::json;
use tempfile::TempDir;
use tower::ServiceExt;

fn setup_router_with_responses(fake_vm_api: FakeVmApi) -> (TempDir, axum::Router) {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = temp_dir.path().join("safepaw.data");
    let db = Arc::new(SafePawDb::open(&db_path).expect("DB should initialize"));
    let fake_vm_api = Arc::new(fake_vm_api);
    let agent_manager = Arc::new(LocalAgentManager::new_with_db(fake_vm_api.clone(), db));
    let state = AppState::new(fake_vm_api.clone(), agent_manager);

    (temp_dir, create_api_router(state))
}

fn setup_router() -> (TempDir, axum::Router) {
    setup_router_with_responses(FakeVmApi::new())
}

async fn response_body_to_json(body: Body) -> serde_json::Value {
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn test_install_agent_success() {
    let (_temp_dir, router) = setup_router_with_responses(FakeVmApi::new().with_exec_response(Ok(
        CommandOutput::success("==> picoclaw installation complete\n"),
    )));

    let request = Request::builder()
        .method("POST")
        .uri("/agents/test-vm/install")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "agent_type": "picoclaw"
            })
            .to_string(),
        ))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_body_to_json(response.into_body()).await;
    assert_eq!(body["success"], true);
    assert!(body["message"].as_str().unwrap().contains("installed"));
}

#[tokio::test]
async fn test_install_agent_failure() {
    let (_temp_dir, router) =
        setup_router_with_responses(FakeVmApi::new().with_exec_response(Ok(CommandOutput {
            status_code: 1,
            stdout: String::new(),
            stderr: "Installation failed\n".to_owned(),
        })));

    let request = Request::builder()
        .method("POST")
        .uri("/agents/test-vm/install")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "agent_type": "picoclaw"
            })
            .to_string(),
        ))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = response_body_to_json(response.into_body()).await;
    assert_eq!(body["success"], false);
    assert!(body["error"].as_str().unwrap().contains("Failed"));
    assert_eq!(body["details"]["code"], "agent_install_failed");
    assert_eq!(body["details"]["operation"], "install_agent");
    assert_eq!(body["details"]["vm_name"], "test-vm");
}

#[tokio::test]
async fn test_check_agent_installed_true() {
    let (_temp_dir, router) =
        setup_router_with_responses(FakeVmApi::new().with_exec_response(Ok(CommandOutput {
            status_code: 0,
            stdout: "/usr/local/bin/picoclaw\n".to_owned(),
            stderr: String::new(),
        })));

    let request = Request::builder()
        .method("POST")
        .uri("/agents/test-vm/check")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "agent_type": "picoclaw"
            })
            .to_string(),
        ))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_body_to_json(response.into_body()).await;
    assert_eq!(body["success"], true);
    assert_eq!(body["installed"], true);
}

#[tokio::test]
async fn test_onboard_agent_success() {
    let (_temp_dir, router) = setup_router_with_responses(
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

    let request = Request::builder()
        .method("POST")
        .uri("/agents/test-vm/onboard")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "my-agent",
                "agent_type": "picoclaw",
                "provider": "openrouter",
                "model": "openrouter/auto",
                "api_key_name": "openrouter-api-key",
                "capabilities": ["filesystem", "network"],
                "max_iterations": 100,
                "workspace_path": "/home/ubuntu/workspace"
            })
            .to_string(),
        ))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response_body_to_json(response.into_body()).await;
    assert_eq!(body["success"], true);
    assert_eq!(body["agent"]["name"], "my-agent");
    assert_eq!(body["agent"]["vm_name"], "test-vm");
    assert_eq!(body["agent"]["status"], "ready");
}

#[tokio::test]
async fn test_onboard_agent_not_installed() {
    let (_temp_dir, router) =
        setup_router_with_responses(FakeVmApi::new().with_exec_response(Ok(CommandOutput {
            status_code: 1,
            stdout: String::new(),
            stderr: String::new(),
        })));

    let request = Request::builder()
        .method("POST")
        .uri("/agents/test-vm/onboard")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "agent_type": "picoclaw",
                "provider": "openrouter",
                "api_key_name": "openrouter-api-key"
            })
            .to_string(),
        ))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = response_body_to_json(response.into_body()).await;
    assert_eq!(body["success"], false);
    assert!(body["error"].as_str().unwrap().contains("not installed"));
    assert_eq!(body["details"]["code"], "agent_onboard_failed");
    assert_eq!(body["details"]["operation"], "onboard_agent");
    assert_eq!(body["details"]["vm_name"], "test-vm");
    assert!(!body["details"]["causes"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_list_and_get_agents_after_onboard() {
    let (_temp_dir, router) = setup_router_with_responses(
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

    let onboard_request = Request::builder()
        .method("POST")
        .uri("/agents/test-vm/onboard")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "test-agent",
                "agent_type": "picoclaw",
                "provider": "openrouter",
                "api_key_name": "openrouter-api-key"
            })
            .to_string(),
        ))
        .unwrap();

    let onboard_response = router.clone().oneshot(onboard_request).await.unwrap();
    let onboard_body = response_body_to_json(onboard_response.into_body()).await;
    let agent_id = onboard_body["agent"]["id"].as_str().unwrap().to_owned();

    let list_request = Request::builder()
        .method("GET")
        .uri("/agents/test-vm")
        .body(Body::empty())
        .unwrap();
    let list_response = router.clone().oneshot(list_request).await.unwrap();

    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body = response_body_to_json(list_response.into_body()).await;
    assert_eq!(list_body["success"], true);
    assert_eq!(list_body["agents"].as_array().unwrap().len(), 1);
    assert_eq!(list_body["agents"][0]["name"], "test-agent");

    let get_request = Request::builder()
        .method("GET")
        .uri(format!("/agents/test-vm/{agent_id}"))
        .body(Body::empty())
        .unwrap();
    let get_response = router.oneshot(get_request).await.unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);
    let get_body = response_body_to_json(get_response.into_body()).await;
    assert_eq!(get_body["success"], true);
    assert_eq!(get_body["agent"]["id"], agent_id);
}

#[tokio::test]
async fn test_full_agent_lifecycle() {
    let (_temp_dir, router) = setup_router_with_responses(
        FakeVmApi::new()
            .with_exec_response(Ok(CommandOutput::success(
                "==> picoclaw installation complete\n",
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

    let install_request = Request::builder()
        .method("POST")
        .uri("/agents/test-vm/install")
        .header("content-type", "application/json")
        .body(Body::from(json!({"agent_type": "picoclaw"}).to_string()))
        .unwrap();
    let install_response = router.clone().oneshot(install_request).await.unwrap();
    assert_eq!(install_response.status(), StatusCode::OK);

    let onboard_request = Request::builder()
        .method("POST")
        .uri("/agents/test-vm/onboard")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "lifecycle-agent",
                "agent_type": "picoclaw",
                "provider": "openrouter",
                "api_key_name": "openrouter-api-key"
            })
            .to_string(),
        ))
        .unwrap();
    let onboard_response = router.clone().oneshot(onboard_request).await.unwrap();
    assert_eq!(onboard_response.status(), StatusCode::CREATED);
    let onboard_body = response_body_to_json(onboard_response.into_body()).await;
    let agent_id = onboard_body["agent"]["id"].as_str().unwrap().to_owned();

    let get_request = Request::builder()
        .method("GET")
        .uri(format!("/agents/test-vm/{agent_id}"))
        .body(Body::empty())
        .unwrap();
    let get_response = router.clone().oneshot(get_request).await.unwrap();
    assert_eq!(get_response.status(), StatusCode::OK);

    let stop_request = Request::builder()
        .method("POST")
        .uri(format!("/agents/test-vm/{agent_id}/stop"))
        .body(Body::empty())
        .unwrap();
    let stop_response = router.clone().oneshot(stop_request).await.unwrap();
    assert_eq!(stop_response.status(), StatusCode::OK);

    let delete_request = Request::builder()
        .method("DELETE")
        .uri(format!("/agents/test-vm/{agent_id}"))
        .body(Body::empty())
        .unwrap();
    let delete_response = router.clone().oneshot(delete_request).await.unwrap();
    assert_eq!(delete_response.status(), StatusCode::OK);

    let list_request = Request::builder()
        .method("GET")
        .uri("/agents/test-vm")
        .body(Body::empty())
        .unwrap();
    let list_response = router.oneshot(list_request).await.unwrap();
    let list_body = response_body_to_json(list_response.into_body()).await;

    assert_eq!(list_body["success"], true);
    assert_eq!(list_body["agents"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_get_agent_not_found() {
    let (_temp_dir, router) = setup_router();

    let request = Request::builder()
        .method("GET")
        .uri("/agents/test-vm/nonexistent-id")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = response_body_to_json(response.into_body()).await;
    assert_eq!(body["success"], false);
    assert!(body["error"].as_str().unwrap().contains("not found"));
    assert_eq!(body["details"]["code"], "agent_get_failed");
    assert_eq!(body["details"]["agent_id"], "nonexistent-id");
}

#[tokio::test]
async fn test_invalid_agent_payload_returns_json_error_details() {
    let (_temp_dir, router) = setup_router();

    let request = Request::builder()
        .method("POST")
        .uri("/agents/test-vm/check")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "agent_type": "zeroclaw"
            })
            .to_string(),
        ))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_body_to_json(response.into_body()).await;
    assert_eq!(body["success"], false);
    assert_eq!(body["details"]["code"], "agent_request_invalid");
    assert_eq!(body["details"]["operation"], "check_agent_installed");
    assert_eq!(body["details"]["vm_name"], "test-vm");
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("Invalid agent request")
    );
}

#[tokio::test]
async fn test_unknown_api_route_returns_json_not_found() {
    let (_temp_dir, router) = setup_router();

    let request = Request::builder()
        .method("POST")
        .uri("/agents/test-vm/unsupported/path")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = response_body_to_json(response.into_body()).await;
    assert_eq!(body["success"], false);
    assert_eq!(body["details"]["code"], "route_not_found");
    assert_eq!(body["details"]["method"], "POST");
    assert_eq!(body["details"]["path"], "/agents/test-vm/unsupported/path");
}

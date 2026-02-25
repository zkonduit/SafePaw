use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use safepaw::vm::{CommandExecutor, CommandOutput, Multipass, MultipassCli};

#[derive(Clone)]
struct FakeExecutor {
    calls: Arc<Mutex<Vec<Vec<String>>>>,
    outputs: Arc<Mutex<VecDeque<CommandOutput>>>,
}

impl FakeExecutor {
    fn new(outputs: Vec<CommandOutput>) -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            outputs: Arc::new(Mutex::new(outputs.into())),
        }
    }

    fn calls(&self) -> Vec<Vec<String>> {
        self.calls.lock().expect("poisoned calls mutex").clone()
    }
}

#[async_trait]
impl CommandExecutor for FakeExecutor {
    async fn run(&self, program: &str, args: &[String]) -> anyhow::Result<CommandOutput> {
        let mut call = Vec::with_capacity(args.len() + 1);
        call.push(program.to_owned());
        call.extend(args.iter().cloned());

        self.calls
            .lock()
            .expect("poisoned calls mutex")
            .push(call);

        self.outputs
            .lock()
            .expect("poisoned outputs mutex")
            .pop_front()
            .ok_or_else(|| anyhow::anyhow!("no fake output available"))
    }
}

#[tokio::test]
async fn launch_info_list_and_stop_flow_maps_to_multipass_commands() {
    let fake = FakeExecutor::new(vec![
        CommandOutput::success(""),
        CommandOutput::success(
            r#"{"errors":[],"info":{"agent-1":{"state":"Running","release":"22.04"}}}"#,
        ),
        CommandOutput::success(
            r#"{"errors":[],"list":[{"name":"agent-1","state":"Running"},{"name":"agent-2","state":"Stopped"}]}"#,
        ),
        CommandOutput::success(""),
    ]);
    let multipass = MultipassCli::new(fake.clone());

    multipass.launch("agent-1").await.expect("launch should work");
    let info = multipass
        .info("agent-1")
        .await
        .expect("info should work");
    let listed = multipass.list().await.expect("list should work");
    multipass.stop("agent-1").await.expect("stop should work");

    assert_eq!(info.name, "agent-1");
    assert_eq!(info.state, "Running");
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].name, "agent-1");
    assert_eq!(listed[0].state, "Running");
    assert_eq!(listed[1].name, "agent-2");
    assert_eq!(listed[1].state, "Stopped");

    assert_eq!(
        fake.calls(),
        vec![
            vec![
                "multipass".to_owned(),
                "launch".to_owned(),
                "--name".to_owned(),
                "agent-1".to_owned()
            ],
            vec![
                "multipass".to_owned(),
                "info".to_owned(),
                "agent-1".to_owned(),
                "--format".to_owned(),
                "json".to_owned()
            ],
            vec![
                "multipass".to_owned(),
                "list".to_owned(),
                "--format".to_owned(),
                "json".to_owned()
            ],
            vec![
                "multipass".to_owned(),
                "stop".to_owned(),
                "agent-1".to_owned()
            ]
        ]
    );
}

#[tokio::test]
async fn launch_returns_error_when_multipass_command_fails() {
    let fake = FakeExecutor::new(vec![CommandOutput {
        status_code: 1,
        stdout: String::new(),
        stderr: "launch failed".to_owned(),
    }]);
    let multipass = MultipassCli::new(fake);

    let err = multipass
        .launch("agent-1")
        .await
        .expect_err("launch should fail");
    assert!(err.to_string().contains("launch"));
    assert!(err.to_string().contains("launch failed"));
}

use std::process::Command;

fn binary_path() -> String {
    std::env::var("NEXTEST_BIN_EXE_safeclaw")
        .or_else(|_| std::env::var("CARGO_BIN_EXE_safeclaw"))
        .or_else(|_| std::env::var("NEXTEST_BIN_EXE_safepaw"))
        .or_else(|_| std::env::var("CARGO_BIN_EXE_safepaw"))
        .unwrap_or_else(|_| "target/debug/safeclaw".to_owned())
}

#[test]
fn cli_displays_help() {
    let output = Command::new(binary_path())
        .arg("--help")
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "expected --help to exit successfully"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"), "expected help usage section");
    assert!(
        stdout.contains("safeclaw") || stdout.contains("safepaw"),
        "expected binary name in help"
    );
}

#[test]
fn cli_displays_help_when_no_args_are_passed() {
    let output = Command::new(binary_path())
        .output()
        .expect("failed to execute binary");

    assert!(
        output.status.success(),
        "expected no-arg invocation to exit successfully"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"), "expected help usage section");
    assert!(
        stdout.contains("safeclaw") || stdout.contains("safepaw"),
        "expected binary name in help"
    );
}

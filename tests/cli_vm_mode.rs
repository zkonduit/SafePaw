use safepaw::cli::{VmMode, build_cli, resolve_vm_mode};

#[test]
fn vm_mode_defaults_to_local() {
    let matches = build_cli()
        .try_get_matches_from(["safeclaw", "vm", "list"])
        .expect("failed to parse CLI args");

    let vm_matches = matches
        .subcommand_matches("vm")
        .expect("missing vm subcommand");
    let mode = resolve_vm_mode(vm_matches).expect("failed to resolve mode");

    assert_eq!(mode, VmMode::Local);
}

#[test]
fn vm_mode_can_be_set_to_network() {
    let matches = build_cli()
        .try_get_matches_from(["safeclaw", "vm", "--mode", "network", "list"])
        .expect("failed to parse CLI args");

    let vm_matches = matches
        .subcommand_matches("vm")
        .expect("missing vm subcommand");
    let mode = resolve_vm_mode(vm_matches).expect("failed to resolve mode");

    assert_eq!(mode, VmMode::Network);
}

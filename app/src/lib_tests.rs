use clap::Parser as _;

use super::*;

fn command_line_launch_mode(arguments: &[&str]) -> LaunchMode {
    let args =
        warp_cli::Args::try_parse_from(std::iter::once("oz").chain(arguments.iter().copied()))
            .expect("arguments should parse");
    let Some(warp_cli::Command::CommandLine(command)) = args.command() else {
        panic!("expected a CLI command: {arguments:?}");
    };
    LaunchMode::command_line(
        command.as_ref().clone(),
        args.global_options().clone(),
        args.debug(),
    )
}

fn computer_use_override(launch_mode: &LaunchMode) -> Option<bool> {
    let LaunchMode::CommandLine {
        computer_use_override,
        ..
    } = launch_mode
    else {
        panic!("expected a CLI launch mode");
    };
    *computer_use_override
}

#[test]
fn agent_run_without_flags_is_unsandboxed_and_leaves_computer_use_to_defaults() {
    let launch_mode = command_line_launch_mode(&["agent", "run", "--prompt", "test"]);

    assert!(!launch_mode.is_sandboxed());
    assert_eq!(computer_use_override(&launch_mode), None);
}

#[test]
fn agent_run_preserves_sandboxed_and_enabled_computer_use() {
    let launch_mode = command_line_launch_mode(&[
        "agent",
        "run",
        "--prompt",
        "test",
        "--sandboxed",
        "--computer-use",
    ]);

    assert!(launch_mode.is_sandboxed());
    assert_eq!(computer_use_override(&launch_mode), Some(true));
}

#[test]
fn agent_run_preserves_disabled_computer_use() {
    let launch_mode =
        command_line_launch_mode(&["agent", "run", "--prompt", "test", "--no-computer-use"]);

    assert!(!launch_mode.is_sandboxed());
    assert_eq!(computer_use_override(&launch_mode), Some(false));
}

#[test]
fn non_agent_command_is_unsandboxed_and_leaves_computer_use_to_defaults() {
    let launch_mode = command_line_launch_mode(&["model", "list"]);

    assert!(!launch_mode.is_sandboxed());
    assert_eq!(computer_use_override(&launch_mode), None);
}

#[test]
fn command_line_launch_runs_in_sdk_execution_mode() {
    let launch_mode = command_line_launch_mode(&["model", "list"]);

    assert_eq!(launch_mode.execution_mode(), ExecutionMode::Sdk);
}

#[test]
fn tui_uses_distinct_secure_storage_service_name() {
    let launch_mode = LaunchMode::Tui {
        entrypoint: TuiEntryPoint::Interactive {
            mount: Box::new(|_| {}),
        },
    };
    assert!(matches!(
        &launch_mode,
        LaunchMode::Tui {
            entrypoint: TuiEntryPoint::Interactive { .. }
        }
    ));

    assert_eq!(
        launch_mode.secure_storage_service_name("dev.warp.Warp-Dev"),
        "dev.warp.Warp-Dev.tui"
    );
}

#[test]
fn app_keeps_default_secure_storage_service_name() {
    let launch_mode = LaunchMode::App {
        args: Default::default(),
    };

    assert_eq!(
        launch_mode.secure_storage_service_name("dev.warp.Warp-Dev"),
        "dev.warp.Warp-Dev"
    );
}

#[test]
fn launch_modes_select_expected_logging_frontend() {
    let tui = LaunchMode::Tui {
        entrypoint: TuiEntryPoint::Interactive {
            mount: Box::new(|_| {}),
        },
    };
    let app = LaunchMode::App {
        args: Default::default(),
    };
    let test = LaunchMode::Test {
        driver: Box::new(None),
        is_integration_test: false,
    };

    assert_eq!(tui.log_frontend(), LogFrontend::Tui);
    assert_eq!(app.log_frontend(), LogFrontend::Gui);
    assert_eq!(test.log_frontend(), LogFrontend::Gui);
    assert_eq!(
        LaunchMode::RemoteServerProxy.log_frontend(),
        LogFrontend::Cli
    );
    assert_eq!(
        LaunchMode::RemoteServerDaemon {
            identity_key: "test".to_owned(),
        }
        .log_frontend(),
        LogFrontend::Cli
    );
}

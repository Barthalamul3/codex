use crate::app_event::ForkPanePlacement;
use codex_core::config::Config;
use codex_core::terminal::Multiplexer;
use codex_protocol::ThreadId;
use codex_protocol::config_types::WebSearchMode;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::SandboxPolicy;
use shlex::try_join;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

pub(crate) struct MultiplexerSpawnConfig {
    pub(crate) program: &'static str,
    pub(crate) args: Vec<String>,
    pub(crate) description: &'static str,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ForkPaneSpawnResult {
    Spawned(&'static str),
    InvalidPlacement(String),
    Failed(String),
}

fn codex_executable() -> PathBuf {
    std::env::current_exe()
        .map(|path| resolve_codex_executable(&path))
        .unwrap_or_else(|_| PathBuf::from("codex"))
}

fn resolve_codex_executable(current_exe: &Path) -> PathBuf {
    let Some(file_name) = current_exe.file_name().and_then(|name| name.to_str()) else {
        return PathBuf::from("codex");
    };
    let Some(base_name) = file_name
        .strip_suffix(".exe")
        .unwrap_or(file_name)
        .strip_prefix("codex-tui")
    else {
        return current_exe.to_path_buf();
    };
    if !base_name.is_empty() {
        return current_exe.to_path_buf();
    }

    let sibling = if file_name.ends_with(".exe") {
        current_exe.with_file_name("codex.exe")
    } else {
        current_exe.with_file_name("codex")
    };

    if sibling.is_file() {
        sibling
    } else {
        PathBuf::from("codex")
    }
}

fn fork_command_parts(
    exe: &Path,
    thread_id: &ThreadId,
    config: &Config,
    additional_writable_roots: &[PathBuf],
) -> Vec<String> {
    let mut args = vec![
        exe.display().to_string(),
        "fork".to_string(),
        "-C".to_string(),
        config.cwd.display().to_string(),
    ];

    if let Some(approval_mode) = approval_mode_arg(config.permissions.approval_policy.value()) {
        args.push("-a".to_string());
        args.push(approval_mode.to_string());
    }
    if let Some(profile) = config.active_profile.as_deref() {
        args.push("-p".to_string());
        args.push(profile.to_string());
    }
    if let Some(model) = config.model.as_deref() {
        args.push("-m".to_string());
        args.push(model.to_string());
    }
    if let Some(sandbox_mode) = sandbox_mode_arg(config.permissions.sandbox_policy.get()) {
        args.push("-s".to_string());
        args.push(sandbox_mode.to_string());
    }
    if config.web_search_mode.value() == WebSearchMode::Live {
        args.push("--search".to_string());
    }
    for root in additional_writable_roots {
        args.push("--add-dir".to_string());
        args.push(root.display().to_string());
    }
    args.push(thread_id.to_string());

    args
}

fn approval_mode_arg(policy: AskForApproval) -> Option<&'static str> {
    match policy {
        AskForApproval::UnlessTrusted => Some("untrusted"),
        AskForApproval::OnFailure => Some("on-failure"),
        AskForApproval::OnRequest => Some("on-request"),
        AskForApproval::Never => Some("never"),
        AskForApproval::Reject(_) => None,
    }
}

fn sandbox_mode_arg(policy: &SandboxPolicy) -> Option<&'static str> {
    match policy {
        SandboxPolicy::DangerFullAccess => Some("danger-full-access"),
        SandboxPolicy::ReadOnly { .. } => Some("read-only"),
        SandboxPolicy::WorkspaceWrite { .. } => Some("workspace-write"),
        SandboxPolicy::ExternalSandbox { .. } => None,
    }
}

fn zellij_direction(placement: ForkPanePlacement) -> Option<&'static str> {
    match placement {
        ForkPanePlacement::Right => Some("right"),
        ForkPanePlacement::Down => Some("down"),
        _ => None,
    }
}

fn build_zellij_new_pane_args(
    command: &[String],
    placement: Option<ForkPanePlacement>,
) -> Vec<String> {
    let mut args = vec![
        "action".to_string(),
        "new-pane".to_string(),
        "--close-on-exit".to_string(),
    ];
    if let Some(placement) = placement {
        if placement == ForkPanePlacement::Float {
            args.push("--floating".to_string());
        } else if let Some(direction) = zellij_direction(placement) {
            args.push("--direction".to_string());
            args.push(direction.to_string());
        } else {
            unreachable!("invalid zellij placement");
        }
    }
    args.push("--".to_string());
    args.extend(command.iter().cloned());
    args
}

fn tmux_split_flags(placement: Option<ForkPanePlacement>) -> [&'static str; 2] {
    match placement {
        None | Some(ForkPanePlacement::Right) => ["-h", ""],
        Some(ForkPanePlacement::Left) => ["-h", "-b"],
        Some(ForkPanePlacement::Down) => ["-v", ""],
        Some(ForkPanePlacement::Up) => ["-v", "-b"],
        _ => unreachable!("invalid tmux placement"),
    }
}

fn build_tmux_new_pane_args(
    command: &[String],
    placement: Option<ForkPanePlacement>,
) -> Vec<String> {
    let command =
        try_join(command.iter().map(String::as_str)).unwrap_or_else(|_| command.join(" "));
    let flags = tmux_split_flags(placement);
    let mut args = vec!["split-window".to_string(), flags[0].to_string()];
    if !flags[1].is_empty() {
        args.push(flags[1].to_string());
    }
    args.push(command);
    args
}

fn fork_spawn_config(
    multiplexer: &Multiplexer,
    exe: &Path,
    thread_id: &ThreadId,
    config: &Config,
    additional_writable_roots: &[PathBuf],
    placement: Option<ForkPanePlacement>,
) -> MultiplexerSpawnConfig {
    let command = fork_command_parts(exe, thread_id, config, additional_writable_roots);
    match multiplexer {
        Multiplexer::Zellij {} => MultiplexerSpawnConfig {
            program: "zellij",
            args: build_zellij_new_pane_args(&command, placement),
            description: "Zellij pane",
        },
        Multiplexer::Tmux { .. } => MultiplexerSpawnConfig {
            program: "tmux",
            args: build_tmux_new_pane_args(&command, placement),
            description: "tmux pane",
        },
    }
}

const TMUX_FLOAT_UNSUPPORTED_MESSAGE: &str = "tmux does not support /fork float.";
const ZELLIJ_UNSUPPORTED_MESSAGE: &str = "Zellij only supports /fork [right|down|float].";
pub(crate) const FORK_PLACEMENT_REQUIRES_MULTIPLEXER_MESSAGE: &str =
    "Fork pane placement requires a terminal multiplexer.";

fn validate_fork_placement_for_multiplexer(
    multiplexer: &Multiplexer,
    placement: Option<ForkPanePlacement>,
) -> Result<(), String> {
    match multiplexer {
        Multiplexer::Zellij {} => match placement {
            None
            | Some(ForkPanePlacement::Right)
            | Some(ForkPanePlacement::Down)
            | Some(ForkPanePlacement::Float) => Ok(()),
            _ => Err(ZELLIJ_UNSUPPORTED_MESSAGE.to_string()),
        },
        Multiplexer::Tmux { .. } => match placement {
            None
            | Some(ForkPanePlacement::Left)
            | Some(ForkPanePlacement::Right)
            | Some(ForkPanePlacement::Up)
            | Some(ForkPanePlacement::Down) => Ok(()),
            _ => Err(TMUX_FLOAT_UNSUPPORTED_MESSAGE.to_string()),
        },
    }
}

pub(crate) async fn spawn_fork_in_new_pane(
    multiplexer: &Multiplexer,
    thread_id: &ThreadId,
    config: &Config,
    additional_writable_roots: &[PathBuf],
    placement: Option<ForkPanePlacement>,
) -> ForkPaneSpawnResult {
    if let Err(err) = validate_fork_placement_for_multiplexer(multiplexer, placement) {
        return ForkPaneSpawnResult::InvalidPlacement(err);
    }
    let exe = codex_executable();
    let spawn_config = fork_spawn_config(
        multiplexer,
        &exe,
        thread_id,
        config,
        additional_writable_roots,
        placement,
    );
    let MultiplexerSpawnConfig {
        program,
        args,
        description,
    } = spawn_config;
    match tokio::task::spawn_blocking(move || {
        Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    })
    .await
    {
        Ok(Ok(_)) => ForkPaneSpawnResult::Spawned(description),
        Ok(Err(err)) => ForkPaneSpawnResult::Failed(format!("failed to run {program}: {err}")),
        Err(err) => ForkPaneSpawnResult::Failed(format!("failed to spawn {program} pane: {err}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_core::config::ConfigBuilder;
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    #[test]
    fn resolve_codex_executable_prefers_sibling_codex_for_codex_tui() {
        let tempdir = tempdir().expect("tempdir");
        let current_exe = tempdir.path().join("codex-tui");
        let sibling = tempdir.path().join("codex");
        std::fs::write(&sibling, b"").expect("create sibling codex");

        assert_eq!(resolve_codex_executable(&current_exe), sibling);
    }

    #[test]
    fn resolve_codex_executable_keeps_non_tui_binary() {
        let current_exe = PathBuf::from("/tmp/codex");

        assert_eq!(resolve_codex_executable(&current_exe), current_exe);
    }

    #[test]
    fn validate_zellij_fork_placement_rejects_left() {
        assert_eq!(
            validate_fork_placement_for_multiplexer(
                &Multiplexer::Zellij {},
                Some(ForkPanePlacement::Left),
            ),
            Err(ZELLIJ_UNSUPPORTED_MESSAGE.to_string())
        );
    }

    #[test]
    fn validate_tmux_fork_placement_rejects_float() {
        assert_eq!(
            validate_fork_placement_for_multiplexer(
                &Multiplexer::Tmux { version: None },
                Some(ForkPanePlacement::Float),
            ),
            Err(TMUX_FLOAT_UNSUPPORTED_MESSAGE.to_string())
        );
    }

    #[tokio::test]
    async fn fork_command_parts_include_current_session_overrides() {
        let codex_home = tempdir().expect("temp codex home");
        let mut config = ConfigBuilder::default()
            .codex_home(codex_home.path().to_path_buf())
            .build()
            .await
            .expect("config");
        config.active_profile = Some("work".to_string());
        config.model = Some("gpt-5".to_string());
        config.cwd = PathBuf::from("/repo");
        config
            .permissions
            .approval_policy
            .set(AskForApproval::OnRequest)
            .expect("approval policy");
        config
            .permissions
            .sandbox_policy
            .set(SandboxPolicy::new_workspace_write_policy())
            .expect("sandbox policy");
        config
            .web_search_mode
            .set(WebSearchMode::Live)
            .expect("web search mode");

        let command = fork_command_parts(
            Path::new("/bin/codex"),
            &ThreadId::new(),
            &config,
            &[PathBuf::from("/extra")],
        );
        let thread_id = command.last().expect("thread id").clone();

        assert_eq!(
            command,
            vec![
                "/bin/codex".to_string(),
                "fork".to_string(),
                "-C".to_string(),
                "/repo".to_string(),
                "-a".to_string(),
                "on-request".to_string(),
                "-p".to_string(),
                "work".to_string(),
                "-m".to_string(),
                "gpt-5".to_string(),
                "-s".to_string(),
                "workspace-write".to_string(),
                "--search".to_string(),
                "--add-dir".to_string(),
                "/extra".to_string(),
                thread_id,
            ]
        );
    }
}

use std::path::Path;

use anyhow::Result;
use predicates::str::contains;
use pretty_assertions::assert_eq;
use tempfile::TempDir;

fn codex_command(codex_home: &Path) -> Result<assert_cmd::Command> {
    let mut cmd = assert_cmd::Command::new(codex_utils_cargo_bin::cargo_bin("codex")?);
    cmd.env("CODEX_HOME", codex_home);
    Ok(cmd)
}

#[tokio::test]
async fn skills_disable_by_path_writes_skill_override_to_config() -> Result<()> {
    let codex_home = TempDir::new()?;
    let skill_path = "/tmp/skills/demo/SKILL.md";

    let mut cmd = codex_command(codex_home.path())?;
    cmd.args(["skills", "disable", "--path", skill_path])
        .assert()
        .success()
        .stdout(contains(format!(
            "Disabled skill path `{skill_path}` in config.toml."
        )));

    let config = std::fs::read_to_string(codex_home.path().join("config.toml"))?;
    let expected = format!("[[skills.config]]\npath = \"{skill_path}\"\nenabled = false\n");
    assert_eq!(config, expected);

    Ok(())
}

#[tokio::test]
async fn skills_enable_by_name_removes_existing_skill_override() -> Result<()> {
    let codex_home = TempDir::new()?;
    std::fs::write(
        codex_home.path().join("config.toml"),
        "[[skills.config]]\nname = \"github:yeet\"\nenabled = false\n",
    )?;

    let mut cmd = codex_command(codex_home.path())?;
    cmd.args(["skills", "enable", "--name", "github:yeet"])
        .assert()
        .success()
        .stdout(contains("Enabled skill `github:yeet` in config.toml."));

    let config = std::fs::read_to_string(codex_home.path().join("config.toml"))?;
    assert_eq!(config, "");

    Ok(())
}

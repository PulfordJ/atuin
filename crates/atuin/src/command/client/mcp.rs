use std::sync::Arc;

use atuin_client::database::Sqlite;
use atuin_client::settings::Settings;
use atuin_common::harnesstools::{AnyHarness, Harness, mcp};
use clap::{Parser, Subcommand};
use eyre::{Result, bail};

use crate::i18n::fl;

#[derive(Subcommand, Debug)]
enum Action {
    #[command(about = fl!("cmd-mcp-install"))]
    Install {
        #[arg(value_name = "AGENT", help = fl!("arg-mcp-install-agent"))]
        agent: Option<String>,
    },
}

/// `atuin mcp` serves over stdio; `atuin mcp install [agent]` registers it.
#[derive(Parser, Debug)]
pub struct Cmd {
    #[command(subcommand)]
    action: Option<Action>,
}

impl Cmd {
    pub async fn run(self, db: &Sqlite, settings: Settings) -> Result<()> {
        match self.action {
            None => Box::pin(atuin_ai::mcp::run(db, Arc::new(settings))).await,
            Some(Action::Install { agent: Some(agent) }) => {
                install(AnyHarness::from_name(&agent)?).await
            }
            Some(Action::Install { agent: None }) => install_all().await,
        }
    }
}

async fn install_all() -> Result<()> {
    let mut found = false;
    let mut failed = Vec::new();
    for &harness in AnyHarness::all() {
        if !mcp::config_exists(harness) {
            continue;
        }
        found = true;
        if let Err(err) = install(harness).await {
            eprintln!("Could not install for {}: {err:#}", harness.name());
            failed.push(harness.name());
        }
    }
    if !found {
        eprintln!(
            "No supported agent config files found. Use `atuin mcp install <agent>` to connect an \
             agent explicitly."
        );
    }
    if !failed.is_empty() {
        bail!("MCP installation failed for: {}", failed.join(", "));
    }
    Ok(())
}

async fn install(harness: AnyHarness) -> Result<()> {
    mcp::install(harness).await?;
    eprintln!("Restart {} to load it.", harness.name());
    eprintln!(
        "To let it search command output and agent threads too, run `atuin config enable \
         output-capture` and `atuin config enable threads`."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::all_agents(None)]
    #[case::explicit(Some("codex"))]
    fn install_accepts_an_optional_agent(#[case] agent: Option<&str>) {
        let mut args = vec!["mcp", "install"];
        args.extend(agent);
        let cmd = Cmd::try_parse_from(args).unwrap();
        let Some(Action::Install { agent: parsed }) = cmd.action else {
            panic!("expected install");
        };
        assert_eq!(parsed.as_deref(), agent);
    }

    #[rstest]
    fn bare_mcp_still_serves() {
        assert!(Cmd::try_parse_from(["mcp"]).unwrap().action.is_none());
    }
}

//! Composition root (ADR-0002): parse the command, read the configuration, start the
//! infrastructure, and hand over to the HTTP or command-line adapter. Nothing else.

use anyhow::Context;
use clap::Parser;
use gite_api::cli::{self, Cli, Command};
use gite_api::config::Config;
use gite_api::http::server;
use gite_api::telemetry;
use gite_api::wiring::{Infrastructure, Migrations};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    // A missing .env file is fine: variables may come from the real environment.
    let _ = dotenvy::dotenv();
    let config = Config::from_env().context("invalid configuration")?;
    telemetry::init(config.log_format);

    match cli.command.unwrap_or(Command::Serve) {
        Command::Serve => {
            let infrastructure = Infrastructure::start(&config, Migrations::BestEffort).await?;
            let state = infrastructure.app_state()?;
            let worker = infrastructure.start_message_worker(&state);
            let result = server::serve(config.bind_addr, state).await;
            worker.abort();
            infrastructure.shutdown().await;
            result
        }
        Command::CreateAdmin {
            email,
            password_stdin,
        } => {
            let infrastructure = Infrastructure::start(&config, Migrations::Required).await?;
            let result =
                cli::run_create_admin(&infrastructure.admin_accounts()?, &email, password_stdin)
                    .await;
            infrastructure.shutdown().await;
            result
        }
    }
}

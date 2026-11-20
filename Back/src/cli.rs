//! Command-line adapter: a driving adapter, like [`crate::http`].
//!
//! It parses the commands, reads the password from the terminal or standard input,
//! calls the use case and turns the result into a message. No business rule lives here.

use std::io::BufRead;

use clap::{Parser, Subcommand};

use crate::application::admin_accounts::{AdminAccounts, CreateAdminError};
use crate::domain::auth::Password;

/// REST API of the La Petite Grange website.
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Available commands; the default is [`Command::Serve`].
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Start the API server (default).
    Serve,
    /// Create an owner account for the admin (password asked twice, hidden).
    CreateAdmin {
        /// Email of the owner.
        #[arg(long)]
        email: String,
        /// Read the password from the first line of standard input (for scripts).
        #[arg(long)]
        password_stdin: bool,
    },
}

/// Where the new password comes from.
pub trait PasswordSource {
    /// Returns the password typed by the operator.
    ///
    /// # Errors
    ///
    /// Returns an error when it cannot be read or the confirmation differs.
    fn read(&self) -> anyhow::Result<Password>;
}

/// Asks the password twice on the terminal, without echo.
#[derive(Debug, Clone, Copy, Default)]
pub struct TerminalPrompt;

impl PasswordSource for TerminalPrompt {
    fn read(&self) -> anyhow::Result<Password> {
        let first = rpassword::prompt_password("Mot de passe : ")?;
        let second = rpassword::prompt_password("Confirmez le mot de passe : ")?;
        anyhow::ensure!(first == second, "les deux mots de passe sont différents");
        Ok(Password::new(first))
    }
}

/// Reads the first line of standard input (end of line removed).
#[derive(Debug, Clone, Copy, Default)]
pub struct StdinLine;

impl PasswordSource for StdinLine {
    fn read(&self) -> anyhow::Result<Password> {
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line)?;
        Ok(Password::new(line.trim_end_matches(['\r', '\n'])))
    }
}

/// Runs `create-admin` from the command line: reads the password from standard input
/// (`--password-stdin`) or the terminal, creates the account and prints the result.
///
/// # Errors
///
/// Returns a user-readable error (invalid email, weak password, duplicate account) or an
/// infrastructure error.
#[allow(clippy::print_stdout)] // the command-line adapter talks to the operator on stdout
pub async fn run_create_admin(
    accounts: &AdminAccounts,
    email: &str,
    password_stdin: bool,
) -> anyhow::Result<()> {
    let source: &dyn PasswordSource = if password_stdin {
        &StdinLine
    } else {
        &TerminalPrompt
    };
    let message = create_admin(accounts, email, source).await?;
    println!("{message}");
    Ok(())
}

/// Creates the account with the password from `source`; returns the confirmation message.
///
/// # Errors
///
/// Returns a user-readable error (invalid email, weak password, duplicate account) or an
/// infrastructure error.
pub async fn create_admin(
    accounts: &AdminAccounts,
    email: &str,
    source: &dyn PasswordSource,
) -> anyhow::Result<String> {
    let password = source.read()?;
    match accounts.create(email, &password).await {
        Ok(_) => Ok(format!("Compte administrateur créé pour {}", email.trim())),
        Err(CreateAdminError::Internal(error)) => Err(error),
        Err(error) => Err(anyhow::anyhow!("{error}")),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::application::auth::fakes::{InMemoryUsers, PlainHasher};

    struct Fixed(&'static str);

    impl PasswordSource for Fixed {
        fn read(&self) -> anyhow::Result<Password> {
            Ok(Password::new(self.0))
        }
    }

    fn accounts() -> AdminAccounts {
        AdminAccounts::new(Arc::new(InMemoryUsers::default()), Arc::new(PlainHasher))
    }

    #[tokio::test]
    async fn prints_a_confirmation() {
        let message = create_admin(
            &accounts(),
            "proprio@example.test",
            &Fixed("correct horse battery"),
        )
        .await
        .unwrap();

        assert_eq!(
            message,
            "Compte administrateur créé pour proprio@example.test"
        );
    }

    #[tokio::test]
    async fn explains_a_weak_password_in_french() {
        let error = create_admin(&accounts(), "proprio@example.test", &Fixed("short"))
            .await
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "le mot de passe doit contenir au moins 12 caractères"
        );
    }

    #[test]
    fn parses_create_admin() {
        let cli = Cli::parse_from(["gite-api", "create-admin", "--email", "a@b.test"]);

        assert!(matches!(
            cli.command,
            Some(Command::CreateAdmin { ref email, password_stdin: false }) if email == "a@b.test"
        ));
    }

    #[test]
    fn serves_by_default() {
        assert!(Cli::parse_from(["gite-api"]).command.is_none());
    }
}

use clap::Parser;

/// tuilist's own Entra ID app registration (a public client; not a secret).
const DEFAULT_CLIENT_ID: &str = "5de297ab-80a5-4cfc-bdea-38e33b917993";

#[derive(Parser)]
#[command(
    name = "todo",
    version,
    about = "A terminal client for Microsoft To Do"
)]
struct Cli {
    /// Entra ID application (client) ID to sign in with; use your own if your
    /// tenant blocks tuilist's.
    #[arg(long, env = "TUILIST_CLIENT_ID", default_value = DEFAULT_CLIENT_ID, global = true)]
    client_id: String,
    /// Tenant to sign in to: consumers (personal accounts), organizations,
    /// common or a tenant ID.
    #[arg(
        long,
        env = "TUILIST_TENANT",
        default_value = "consumers",
        global = true
    )]
    tenant: String,
    /// Browse built-in sample lists instead of signing in.
    #[arg(long)]
    demo: bool,
    /// Seconds between syncs; 0 syncs only at start and on F5.
    #[arg(long, env = "TUILIST_SYNC_EVERY", default_value_t = 60, global = true)]
    sync_every: u64,
    /// Don't ask GitHub for a newer tuilist at start.
    #[arg(
        long,
        env = "TUILIST_NO_UPDATE_CHECK",
        value_parser = clap::builder::FalseyValueParser::new()
    )]
    no_update_check: bool,
}

fn main() -> anyhow::Result<()> {
    let _cli = Cli::parse();
    anyhow::bail!("Not signed in. Run todo login, or todo --demo to look around.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signs_in_with_the_tuilist_app_registration_by_default() {
        let cli = Cli::try_parse_from(["todo"]).unwrap();

        assert_eq!(
            cli.client_id.as_str(),
            "5de297ab-80a5-4cfc-bdea-38e33b917993"
        );
    }

    #[test]
    fn signs_in_to_personal_accounts_by_default() {
        let cli = Cli::try_parse_from(["todo"]).unwrap();

        assert_eq!(cli.tenant, "consumers");
    }

    #[test]
    fn the_client_id_can_be_overridden() {
        let cli = Cli::try_parse_from(["todo", "--client-id", "my-own-app"]).unwrap();

        assert_eq!(cli.client_id, "my-own-app");
    }

    #[test]
    fn syncs_every_minute_unless_told_otherwise() {
        let cli = Cli::try_parse_from(["todo"]).unwrap();
        assert_eq!(cli.sync_every, 60);

        let cli = Cli::try_parse_from(["todo", "--sync-every", "0"]).unwrap();
        assert_eq!(cli.sync_every, 0);
    }

    #[test]
    fn the_update_check_can_be_turned_off() {
        let cli = Cli::try_parse_from(["todo", "--no-update-check"]).unwrap();

        assert!(cli.no_update_check);
    }
}

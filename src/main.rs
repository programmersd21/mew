use std::io::IsTerminal;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use mew::{git, hook, render, telemetry, theme};

#[derive(Parser, Debug)]
#[command(
    name = "mew",
    version,
    about = "fast, zero-daemon terminal companion and git workspace dashboard"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// print the default theme.toml (syntax highlighted on a tty, raw when piped)
    Theme {
        /// print raw toml even on a tty (for docs and scripts)
        #[arg(long)]
        raw: bool,
    },
    /// print shell hook snippet for automatic invocation on cd
    Hook {
        /// target shell (bash, zsh, fish, nu, powershell)
        shell: hook::Shell,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Theme { raw }) => {
            // the piped path must stay byte-clean toml: no ansi, no wordmark, no
            // hints — `mew theme > theme.toml` has to write a valid file.
            let source = theme::default_theme_toml();
            if raw || !std::io::stdout().is_terminal() {
                print!("{}", source);
                return Ok(());
            }
            let theme_config = theme::load_theme();
            let palette = theme::Palette::from_config(&theme_config, theme::detect_color_mode());
            print!("{}", theme::highlight_toml(&source, &palette));
            println!();
            println!("  {}", palette.color_palette_dots());
            match theme::theme_path() {
                Some(path) => eprintln!("mew: tip: mew theme > {}", path.display()),
                None => eprintln!("mew: tip: mew theme > ~/.config/mew/theme.toml"),
            }
        }
        Some(Commands::Hook { shell }) => hook::print_hook(shell),
        None => {
            let color_mode = theme::detect_color_mode();
            let theme_config = theme::load_theme();
            let palette = theme::Palette::from_config(&theme_config, color_mode);

            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let git_state = git::detect_git_state(&cwd).unwrap_or_default();

            let top_lang = telemetry::detect_project_language(&cwd);
            let output = render::render_compact(&git_state, &palette, top_lang.as_deref());
            print!("{}", output);
        }
    }

    Ok(())
}

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use mew::{git, hook, render, system, telemetry, theme};

#[derive(Parser, Debug)]
#[command(
    name = "mew",
    version,
    about = "fast, zero-daemon terminal companion and git workspace dashboard"
)]
struct Cli {
    /// render full multi-pane dashboard with system health and project telemetry
    #[arg(long)]
    full: bool,

    /// counts lines if in non-git directory
    #[arg(long, short)]
    force_count_lines: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// print shell hook snippet for automatic invocation on cd
    Hook {
        /// target shell (bash, zsh, fish, nu, powershell)
        shell: hook::Shell,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(Commands::Hook { shell }) = cli.command {
        hook::print_hook(shell);
        return Ok(());
    }

    let color_mode = theme::detect_color_mode();
    let theme_config = theme::load_theme();
    let palette = theme::Palette::from_config(&theme_config, color_mode);

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let git_state = git::detect_git_state(&cwd).unwrap_or_default();

    if cli.full {
        let sys_telem = system::collect_system_telemetry();
        let proj_telem = telemetry::collect_project_telemetry(cli.force_count_lines, &cwd);
        let output = render::render_full(
            cli.force_count_lines,
            &git_state,
            &sys_telem,
            &proj_telem,
            &palette,
        );
        print!("{}", output);
    } else {
        let top_lang = telemetry::detect_project_language(&cwd);
        let output = render::render_compact(&git_state, &palette, top_lang.as_deref());
        print!("{}", output);
    }

    Ok(())
}

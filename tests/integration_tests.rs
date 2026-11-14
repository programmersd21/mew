use std::path::PathBuf;

use mew::git::{GitAheadBehind, GitState, MascotState, compute_mascot_state};
use mew::render::{
    format_git_pill, gradient_text, render_compact, render_gauge_porcelain, time_widget_strings,
    truncate_visible, visible_width,
};

#[test]
fn test_mascot_priority_resolution() {
    // 1. Conflict priority: highest
    let mut git = GitState {
        is_repo: true,
        repo_root: Some(PathBuf::from("/repo")),
        branch_name: "main".to_string(),
        commit_hash: "a1b2c3d".to_string(),
        is_detached: false,
        ahead_behind: GitAheadBehind {
            ahead: 2,
            behind: 0,
            has_upstream: true,
            upstream_name: Some("origin/main".to_string()),
        },
        modified_count: 3,
        staged_count: 1,
        untracked_count: 2,
        conflict_count: 1,
    };
    assert_eq!(compute_mascot_state(&git, true), MascotState::Conflict);

    // 2. Dirty priority (modified / untracked)
    git.conflict_count = 0;
    assert_eq!(compute_mascot_state(&git, true), MascotState::Dirty);

    // 3. Staged-only priority
    git.modified_count = 0;
    git.untracked_count = 0;
    assert_eq!(compute_mascot_state(&git, true), MascotState::StagedOnly);

    // 4. Ahead-only priority
    git.staged_count = 0;
    assert_eq!(compute_mascot_state(&git, true), MascotState::AheadOnly);

    // 5. Clean with time widget
    git.ahead_behind.ahead = 0;
    assert_eq!(compute_mascot_state(&git, true), MascotState::CleanWithTime);

    // 6. Clean without time widget
    assert_eq!(compute_mascot_state(&git, false), MascotState::CleanNoTime);

    // 7. No repo
    git.is_repo = false;
    assert_eq!(compute_mascot_state(&git, false), MascotState::NoRepo);
}

#[test]
fn test_git_pill_formatting() {
    let palette = mew::theme::Palette::from_config(
        &mew::theme::ThemeConfig::default(),
        mew::theme::ColorMode::Truecolor,
    );
    let mut git = GitState {
        is_repo: true,
        repo_root: None,
        branch_name: "main".to_string(),
        commit_hash: "a1b2c3d".to_string(),
        is_detached: false,
        ahead_behind: GitAheadBehind {
            ahead: 0,
            behind: 0,
            has_upstream: false,
            upstream_name: None,
        },
        modified_count: 0,
        staged_count: 0,
        untracked_count: 0,
        conflict_count: 0,
    };

    let pill = format_git_pill(&git, &palette);
    assert!(pill.contains("main"));
    assert!(pill.contains("clean"));

    git.modified_count = 2;
    git.staged_count = 1;
    let pill = format_git_pill(&git, &palette);
    assert!(pill.contains("2 modified"));
    assert!(pill.contains("1 staged"));
}

#[test]
fn test_time_widget_strings_are_lowercase_and_bounded() {
    let (hm, date, tz, day_pct) = time_widget_strings();
    assert_eq!(hm, hm.to_lowercase());
    assert_eq!(date, date.to_lowercase());
    assert_eq!(tz, tz.to_lowercase());
    assert!(hm.contains(':'));
    assert!((0.0..=100.0).contains(&day_pct));
}

#[test]
fn test_porcelain_gauge_is_exact_width_single_texture() {
    // 25% of 10 rounds to 3 filled + 7 track, all full-height blocks
    let g = render_gauge_porcelain(25.0, 10, "", "", "");
    assert_eq!(g, "██████████");
    assert_eq!(visible_width(&g), 10);
    // markers ride along without counting toward width
    let marked = render_gauge_porcelain(25.0, 10, "F", "E", "R");
    assert_eq!(marked, "F███E███████R");
    assert_eq!(visible_width(&marked), 13);
    let full = render_gauge_porcelain(100.0, 8, "", "", "");
    assert_eq!(visible_width(&full), 8);
    let empty = render_gauge_porcelain(0.0, 8, "", "", "");
    assert_eq!(visible_width(&empty), 8);
}

#[test]
fn test_custom_theme_values_flow_into_render() {
    let colors = mew::theme::ThemeColors {
        dot_workspace: "#ff0000".to_string(),
        title_mew: "#00ff00".to_string(),
        ..Default::default()
    };
    let config = mew::theme::ThemeConfig { colors };
    let palette = mew::theme::Palette::from_config(&config, mew::theme::ColorMode::Truecolor);
    // workspace icon uses the custom dot color (255,0,0), not the default
    assert!(palette.dot_workspace.contains("38;2;255;0;0"));
    // gradient stops come from config too
    assert_eq!(palette.dot_workspace_hex, "#ff0000");
    assert_eq!(palette.title_mew_hex, "#00ff00");
    // and the rendered card actually paints it
    let git = mew::git::GitState::default();
    let out = mew::render::render_compact(&git, &palette, None);
    assert!(out.contains("38;2;255;0;0"));
}

#[test]
fn test_legacy_theme_toml_still_parses() {
    // pre-accent theme files (14 keys) must keep loading with defaults
    let old = "[colors]\nborder = \"#111111\"\n";
    let cfg: mew::theme::ThemeConfig = toml::from_str(old).unwrap();
    assert_eq!(cfg.colors.border, "#111111");
    assert_eq!(cfg.colors.dot_workspace, "#89dceb");
    assert_eq!(cfg.colors.title_mew, "#cba6f7");
    assert_eq!(cfg.colors.dots.len(), 8);
}

#[test]
fn test_default_theme_toml_roundtrips_to_default_config() {
    let src = mew::theme::default_theme_toml();
    // never contains escape bytes: the piped path must stay clean toml
    assert!(!src.contains('\x1b'));
    assert!(src.contains("[colors]"));
    // every public key ships in the template
    for key in [
        "border",
        "primary",
        "muted",
        "mascot",
        "task_active",
        "gauge_fill",
        "gauge_empty",
        "status_clean",
        "status_dirty",
        "status_error",
        "dot_workspace",
        "dot_git",
        "dot_env",
        "dot_task",
        "dot_mood",
        "title_mew",
        "dots",
    ] {
        assert!(src.contains(key), "default theme.toml missing key {}", key);
    }
    // the generated file parses and equals the built-in palette exactly
    let parsed: mew::theme::ThemeConfig = toml::from_str(&src).unwrap();
    assert_eq!(parsed, mew::theme::ThemeConfig::default());
}

#[test]
fn test_highlight_toml_plain_passthrough_and_ansi_strip_roundtrip() {
    let cfg = mew::theme::ThemeConfig::default();
    let src = mew::theme::default_theme_toml();
    // plain mode: byte-identical passthrough
    let plain = mew::theme::Palette::from_config(&cfg, mew::theme::ColorMode::Plain);
    assert_eq!(mew::theme::highlight_toml(&src, &plain), src);
    // truecolor: escapes added, but stripping them recovers the source exactly
    let palette = mew::theme::Palette::from_config(&cfg, mew::theme::ColorMode::Truecolor);
    let highlighted = mew::theme::highlight_toml(&src, &palette);
    assert!(highlighted.contains("\x1b["));
    let mut stripped = String::new();
    let mut in_escape = false;
    for c in highlighted.chars() {
        if c == '\x1b' {
            in_escape = true;
            continue;
        }
        if in_escape {
            if c == 'm' {
                in_escape = false;
            }
            continue;
        }
        stripped.push(c);
    }
    assert_eq!(stripped, src);
}

#[test]
fn test_style_clap_output_plain_passthrough_and_ansi_strip() {
    let cfg = mew::theme::ThemeConfig::default();
    let src = "error: unrecognized subcommand 'hoook'\n\nUsage: mew [COMMAND]\n";
    let plain = mew::theme::Palette::from_config(&cfg, mew::theme::ColorMode::Plain);
    let passthrough = mew::clap_ui::style_clap_output(src, mew::clap_ui::ClapScreen::Error, &plain);
    assert!(!passthrough.contains('\x1b'));
    assert_eq!(
        passthrough,
        "error: unrecognized subcommand 'hoook'\n\nusage: mew [COMMAND]\n"
    );

    let palette = mew::theme::Palette::from_config(&cfg, mew::theme::ColorMode::Truecolor);
    let styled = mew::clap_ui::style_clap_output(src, mew::clap_ui::ClapScreen::Error, &palette);
    assert!(styled.contains("\x1b["));
    // the guidance is unaltered once the escapes come off
    let mut stripped = String::new();
    let mut in_escape = false;
    for c in styled.chars() {
        if c == '\x1b' {
            in_escape = true;
            continue;
        }
        if in_escape {
            if c == 'm' {
                in_escape = false;
            }
            continue;
        }
        stripped.push(c);
    }
    assert_eq!(stripped, passthrough);
}

#[test]
fn test_unknown_subcommand_error_is_themed_and_plain_when_piped() {
    let bin = env!("CARGO_BIN_EXE_mew");
    let out = std::process::Command::new(bin)
        .arg("hoook")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty(), "errors belong on stderr");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("unrecognized subcommand"));
    // the corrective tip survives styling
    assert!(stderr.contains("hook"));
    assert!(stderr.contains("usage:"));
    assert!(!stderr.contains('\x1b'), "piped errors must carry no ansi");
}

#[test]
fn test_missing_required_arg_error_is_themed_and_plain_when_piped() {
    let bin = env!("CARGO_BIN_EXE_mew");
    let out = std::process::Command::new(bin)
        .arg("hook")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("<SHELL>"));
    assert!(stderr.contains("usage:"));
    assert!(!stderr.contains('\x1b'));
}

#[test]
fn test_unexpected_flag_error_is_themed_and_plain_when_piped() {
    let bin = env!("CARGO_BIN_EXE_mew");
    let out = std::process::Command::new(bin).arg("-Z").output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("error:"));
    assert!(!stderr.contains('\x1b'));
}

#[test]
fn test_help_output_is_branded_and_lowercase_when_piped() {
    let bin = env!("CARGO_BIN_EXE_mew");
    let out = std::process::Command::new(bin)
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.starts_with("mew · "), "about line is branded");
    assert!(stdout.contains("usage:"));
    assert!(stdout.contains("commands:"));
    assert!(stdout.contains("theme"));
    assert!(stdout.contains("hook"));
    assert!(!stdout.contains('\x1b'));
    // subcommand help gets the same treatment
    let out = std::process::Command::new(bin)
        .args(["hook", "--help"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("<SHELL>"));
    assert!(!stdout.contains('\x1b'));
}

#[test]
fn test_version_output_carries_version_string() {
    let bin = env!("CARGO_BIN_EXE_mew");
    let out = std::process::Command::new(bin)
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
    assert!(!stdout.contains('\x1b'));
}

#[test]
fn test_theme_subcommand_pipes_raw_toml_and_accepts_raw_flag() {
    let bin = env!("CARGO_BIN_EXE_mew");
    for args in [vec!["theme"], vec!["theme", "--raw"]] {
        let out = std::process::Command::new(bin)
            .args(&args)
            .output()
            .unwrap();
        assert!(out.status.success(), "mew theme exited non-zero");
        // not a tty under process spawn: stdout must be pure toml, stderr silent
        assert!(!out.stdout.is_empty());
        assert!(
            out.stderr.is_empty(),
            "piped mew theme leaked to stderr: {:?}",
            String::from_utf8_lossy(&out.stderr)
        );
        let src = String::from_utf8(out.stdout).unwrap();
        assert!(!src.contains('\x1b'));
        let parsed: mew::theme::ThemeConfig = toml::from_str(&src).unwrap();
        assert_eq!(parsed, mew::theme::ThemeConfig::default());
    }
}

#[test]
fn test_gradient_text_plain_mode_passthrough() {
    let palette = mew::theme::Palette::from_config(
        &mew::theme::ThemeConfig::default(),
        mew::theme::ColorMode::Plain,
    );
    assert_eq!(
        mew::render::gradient_text("14:05", "#74c7ec", "#cba6f7", &palette),
        "14:05"
    );
}
#[test]
fn test_gradient_text_emits_per_char_truecolor() {
    let palette = mew::theme::Palette::from_config(
        &mew::theme::ThemeConfig::default(),
        mew::theme::ColorMode::Truecolor,
    );
    let g = gradient_text("12:34", "#89dceb", "#cba6f7", &palette);
    assert_eq!(visible_width(&g), 5);
    assert!(g.contains("\x1b[38;2;"));
}

#[test]
fn test_truncate_visible_caps_width_and_keeps_ansi_intact() {
    // plain text cuts at exactly max
    assert_eq!(truncate_visible("hello world", 5), "hello");
    assert_eq!(visible_width(&truncate_visible("hello world", 5)), 5);
    // short text passes through untouched
    assert_eq!(truncate_visible("hi", 5), "hi");
    // ansi escapes ride along without counting toward width
    let colored = "\x1b[38;2;137;180;250mhi\x1b[0m there";
    let cut = truncate_visible(colored, 5);
    assert_eq!(visible_width(&cut), 5);
    assert!(cut.contains("\x1b[38;2;137;180;250m"));
}

fn reaction_git_state() -> GitState {
    GitState {
        is_repo: true,
        repo_root: Some(PathBuf::from("/repo")),
        branch_name: "main".to_string(),
        commit_hash: "a1b2c3d".to_string(),
        is_detached: false,
        ahead_behind: GitAheadBehind {
            ahead: 0,
            behind: 0,
            has_upstream: false,
            upstream_name: None,
        },
        modified_count: 0,
        staged_count: 0,
        untracked_count: 0,
        conflict_count: 0,
    }
}

// every mascot state must paint its own face + chip word and hold the
// frame: 72 cols in every line. no shared fallbacks.
#[test]
fn test_all_mascot_reactions_render() {
    let palette = mew::theme::Palette::from_config(
        &mew::theme::ThemeConfig::default(),
        mew::theme::ColorMode::Truecolor,
    );

    let mut conflict = reaction_git_state();
    conflict.conflict_count = 1;
    let mut dirty = reaction_git_state();
    dirty.modified_count = 2;
    let mut staged = reaction_git_state();
    staged.staged_count = 1;
    let mut ahead = reaction_git_state();
    ahead.ahead_behind = GitAheadBehind {
        ahead: 3,
        behind: 0,
        has_upstream: true,
        upstream_name: Some("origin/main".to_string()),
    };
    let clean = reaction_git_state();
    let mut norepo = reaction_git_state();
    norepo.is_repo = false;

    // (git state, expected chip word, expected face). note: the renderers
    // always pass has_time_widget=true, so CleanNoTime is covered by the
    // priority unit test above, not here.
    let cases: Vec<(GitState, &str, &str)> = vec![
        (conflict, "conflict", "( >_< )"),
        (dirty, "watching", "( o.o )"),
        (staged, "ready", "( ~_~ )"),
        (ahead, "ship", "( ^.^)~"),
        (clean, "ticking", "( -.~ )"),
        (norepo, "no repo", "( =.= )"),
    ];

    for (git, word, face) in &cases {
        let out = render_compact(git, &palette, None);
        assert!(out.contains(word), "missing chip word '{}'", word);
        assert!(out.contains(face), "missing face '{}'", face);
        for line in out.lines() {
            assert_eq!(
                mew::render::visible_width(line),
                72,
                "line off width in '{}' state",
                word
            );
        }
    }
}

#[test]
fn test_toolchain_probe_rust_and_unknown() {
    // rustc is guaranteed present (dev shells and ci install the toolchain)
    let rust = mew::system::read_toolchain_version("rust");
    assert!(rust.is_some());
    let rust = rust.unwrap();
    assert_eq!(rust, rust.to_lowercase());
    assert!(rust.starts_with("rustc "));
    // kotlin needs a jvm boot: always none. unknown langs: none.
    assert!(mew::system::read_toolchain_version("kotlin").is_none());
    assert!(mew::system::read_toolchain_version("cobol").is_none());
}

#[test]
fn test_detect_project_language_markers() {
    let base = std::env::temp_dir().join(format!("mew_lang_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    std::fs::write(base.join("Cargo.toml"), "[package]\n").unwrap();
    assert_eq!(
        mew::telemetry::detect_project_language(&base).as_deref(),
        Some("rust")
    );
    std::fs::remove_file(base.join("Cargo.toml")).unwrap();
    std::fs::write(base.join("package.json"), "{}\n").unwrap();
    std::fs::write(base.join("tsconfig.json"), "{}\n").unwrap();
    assert_eq!(
        mew::telemetry::detect_project_language(&base).as_deref(),
        Some("typescript")
    );
    std::fs::remove_file(base.join("package.json")).unwrap();
    std::fs::remove_file(base.join("tsconfig.json")).unwrap();
    assert_eq!(mew::telemetry::detect_project_language(&base), None);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn test_env_string_is_lowercase_with_fallbacks() {
    let s = mew::render::format_env_string(Some("cobol"));
    assert_eq!(s, s.to_lowercase());
    assert!(!s.is_empty());
    let s = mew::render::format_env_string(Some("rust"));
    assert_eq!(s, s.to_lowercase());
    assert!(s.contains("rustc"));
}

#[test]
fn test_detect_git_state_ahead_tracking() {
    // live libgit2 exercise of the ahead/behind lookup: real repos, tempdir only
    let base = std::env::temp_dir().join(format!("mew_git_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let remote = base.join("up.git");
    let work = base.join("wc");
    std::fs::create_dir_all(&work).unwrap();
    let git = |dir: &std::path::Path, args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .output()
            .unwrap()
    };
    assert!(git(&work, &["init", "-b", "main"]).status.success());
    assert!(
        git(&work, &["init", "--bare", remote.to_str().unwrap()])
            .status
            .success()
    );
    std::fs::write(work.join("a.txt"), "one\n").unwrap();
    assert!(git(&work, &["add", "."]).status.success());
    assert!(git(&work, &["commit", "-m", "one"]).status.success());
    assert!(
        git(
            &work,
            &["remote", "add", "origin", remote.to_str().unwrap()]
        )
        .status
        .success()
    );
    assert!(
        git(&work, &["push", "-u", "origin", "main"])
            .status
            .success()
    );
    std::fs::write(work.join("b.txt"), "two\n").unwrap();
    assert!(git(&work, &["add", "."]).status.success());
    assert!(git(&work, &["commit", "-m", "two"]).status.success());

    let state = mew::git::detect_git_state(&work).unwrap();
    assert!(state.is_repo);
    assert_eq!(state.branch_name, "main");
    assert_eq!(state.commit_hash.len(), 7);
    assert!(state.ahead_behind.has_upstream);
    assert_eq!(
        (state.ahead_behind.ahead, state.ahead_behind.behind),
        (1, 0)
    );
    assert!(
        state
            .ahead_behind
            .upstream_name
            .as_deref()
            .unwrap_or("")
            .contains("origin/main")
    );

    let _ = std::fs::remove_dir_all(&base);
}

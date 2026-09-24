use std::path::PathBuf;

use mew::git::{GitAheadBehind, GitState, MascotState, compute_mascot_state};
use mew::render::{
    format_branch_info, format_git_pill, gradient_text, render_compact, render_full,
    render_gauge_porcelain, time_widget_strings, truncate_visible, visible_width,
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
fn test_branch_info_formatting() {
    let mut git = GitState::default();

    // not a repo
    assert_eq!(format_branch_info(&git), "not a git repository");

    git.is_repo = true;
    git.branch_name = "feat/lexer".to_string();

    // no upstream
    assert_eq!(format_branch_info(&git), "no upstream configured");

    // detached head
    git.is_detached = true;
    git.branch_name = "detached@a1b2c3d".to_string();
    assert_eq!(
        format_branch_info(&git),
        "detached at head (detached@a1b2c3d)"
    );

    // clean upstream up to date
    git.is_detached = false;
    git.branch_name = "main".to_string();
    git.ahead_behind = GitAheadBehind {
        ahead: 0,
        behind: 0,
        has_upstream: true,
        upstream_name: Some("origin/main".to_string()),
    };
    assert_eq!(format_branch_info(&git), "up to date with origin/main");

    // ahead only
    git.ahead_behind.ahead = 3;
    assert_eq!(format_branch_info(&git), "3 ahead of origin/main");

    // behind only
    git.ahead_behind.ahead = 0;
    git.ahead_behind.behind = 2;
    assert_eq!(format_branch_info(&git), "2 behind origin/main");

    // ahead and behind
    git.ahead_behind.ahead = 4;
    git.ahead_behind.behind = 1;
    assert_eq!(format_branch_info(&git), "4 ahead, 1 behind origin/main");
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
fn test_shell_info_is_lowercase_and_nonempty() {
    let (shell, term) = mew::system::read_shell_info();
    assert_eq!(shell, shell.to_lowercase());
    assert_eq!(term, term.to_lowercase());
    assert!(!shell.is_empty() && !term.is_empty());
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
// frame: 72 cols in tiny mode, 79 in full mode. no shared fallbacks.
#[test]
fn test_all_mascot_reactions_render() {
    let palette = mew::theme::Palette::from_config(
        &mew::theme::ThemeConfig::default(),
        mew::theme::ColorMode::Truecolor,
    );
    let sys = mew::system::SystemTelemetry::default();
    let telem = mew::telemetry::ProjectTelemetry::default();

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
        // tiny mode: word + face present, every line exactly 72
        let tiny = render_compact(git, &palette, None);
        assert!(
            tiny.contains(word),
            "tiny mode missing chip word '{}'",
            word
        );
        assert!(tiny.contains(face), "tiny mode missing face '{}'", face);
        for line in tiny.lines() {
            assert_eq!(
                mew::render::visible_width(line),
                72,
                "tiny mode line off width in '{}' state",
                word
            );
        }

        // full mode drives the same face from the same state
        let full = render_full(true, git, &sys, &telem, &palette);
        assert!(
            full.contains(word),
            "full mode missing chip word '{}'",
            word
        );
        assert!(full.contains(face), "full mode missing face '{}'", face);
        for line in full.lines() {
            assert_eq!(
                mew::render::visible_width(line),
                79,
                "full mode line off width in '{}' state",
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

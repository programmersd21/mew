use std::path::Path;

use crate::git::{GitState, compute_mascot_state, mascot_visual_for_state};
use crate::system::{read_os_name, read_rustc_version, read_toolchain_version, read_uptime};
use crate::theme::Palette;

const LABEL_WIDTH: usize = 9;
const MASCOT_WIDTH: usize = 14;

fn label(p: &Palette, text: &str) -> String {
    format!(
        "{}{:<width$}{}",
        p.muted,
        text,
        p.reset,
        width = LABEL_WIDTH
    )
}

fn border_line(
    p: &Palette,
    total_width: usize,
    left: char,
    right: char,
    label_text: Option<&str>,
) -> String {
    match label_text {
        None => format!(
            "{}{}{}{}{}\n",
            p.border,
            left,
            "─".repeat(total_width.saturating_sub(2)),
            right,
            p.reset
        ),
        // wordmark painted with the signature sky→mauve gradient; plain mode
        // degrades to bare text via gradient_text's passthrough.
        Some(text) => {
            let dashes = total_width.saturating_sub(5 + text.chars().count());
            let word = gradient_text(text, &p.dot_workspace_hex, &p.title_mew_hex, p);
            format!(
                "{}{}─{} {}{}{} {}{}{}{}\n",
                p.border,
                left,
                p.reset,
                p.bold,
                word,
                p.reset,
                p.border,
                "─".repeat(dashes),
                right,
                p.reset
            )
        }
    }
}

pub fn format_workspace_path(path: Option<&Path>) -> String {
    let p = match path {
        Some(p) => p.to_path_buf(),
        None => match std::env::current_dir() {
            Ok(c) => c,
            Err(_) => return "unknown".to_string(),
        },
    };

    let canonical = p.canonicalize().unwrap_or(p);
    let raw = canonical.to_string_lossy();
    let stripped = raw.strip_prefix(r"\\?\").unwrap_or(&raw);
    let normalized = if cfg!(windows) {
        stripped.replace('\\', "/")
    } else {
        stripped.to_string()
    };
    let path_str = normalized;

    if let Some(home) = dirs::home_dir() {
        let home_raw = home.to_string_lossy();
        let home_stripped = home_raw.strip_prefix(r"\\?\").unwrap_or(&home_raw);
        let home_normalized = if cfg!(windows) {
            home_stripped.replace('\\', "/")
        } else {
            home_stripped.to_string()
        };
        if path_str.starts_with(home_normalized.as_str()) {
            let relative = &path_str[home_normalized.len()..];
            return format!("~{}", relative).to_lowercase();
        }
    }

    path_str.to_string().to_lowercase()
}

// git status pill with per-segment accent colors. the row already carries the
// branch icon, so the pill starts with the bare name (no dupes). every icon
// is a private-use nerd font glyph (single cell in all patched fonts):
// \uf0e7 fa-bolt, \uf00c fa-check. U+26A1 is deliberately avoided — it can
// resolve to a 2-cell emoji presentation and dislocate the right border.
// branch name lowercased.
pub fn format_git_pill(git: &GitState, p: &Palette) -> String {
    if !git.is_repo {
        return format!("{}no repo{}", p.muted, p.reset);
    }

    const BOLT: char = '\u{f0e7}';
    const CHECK: char = '\u{f00c}';
    let branch = git.branch_name.to_lowercase();
    let sep = format!("{} • {}", p.muted, p.reset);
    let mut segs = vec![format!("{}{}{}", p.dot_git, branch, p.reset)];
    if git.conflict_count > 0 {
        segs.push(format!(
            "{}{} {} conflict{}",
            p.status_error, BOLT, git.conflict_count, p.reset
        ));
    }
    let dirty = git.modified_count + git.untracked_count;
    if dirty > 0 {
        segs.push(format!(
            "{}{} {} modified{}",
            p.status_dirty, BOLT, dirty, p.reset
        ));
    }
    if git.staged_count > 0 {
        segs.push(format!(
            "{}{} {} staged{}",
            p.status_clean, CHECK, git.staged_count, p.reset
        ));
    }
    if segs.len() == 1 {
        segs.push(format!("{}clean{}", p.status_clean, p.reset));
    }

    segs.join(&sep)
}

// env row: os plus the toolchain of the active project language, probed
// live. falls back to rustc, then to the bare os name. all lowercase.
pub fn format_env_string(top_lang: Option<&str>) -> String {
    let os = read_os_name().to_lowercase();
    let toolchain = top_lang
        .and_then(read_toolchain_version)
        .or_else(read_rustc_version);
    match toolchain {
        Some(v) => format!("{} • {}", os, v.to_lowercase()),
        None => os,
    }
}

pub fn render_gauge_porcelain(
    percent: f32,
    width: usize,
    active_color: &str,
    track_color: &str,
    reset: &str,
) -> String {
    let clamped = percent.clamp(0.0, 100.0);
    let filled_count = ((clamped / 100.0) * (width as f32)).round() as usize;
    let empty_count = width.saturating_sub(filled_count);

    format!(
        "{}{}{}{}{}",
        active_color,
        "█".repeat(filled_count),
        track_color,
        "█".repeat(empty_count),
        reset
    )
}

// smooth brand gradient across short text (stripe-style): per-character
// interpolation between two catppuccin hex stops. plain mode returns the
// text untouched so piped output stays clean. no trailing reset emitted;
// callers close with their own reset.
pub fn gradient_text(text: &str, start_hex: &str, end_hex: &str, p: &Palette) -> String {
    if p.is_plain {
        return text.to_string();
    }
    let (r1, g1, b1) = crate::theme::hex_to_rgb(start_hex);
    let (r2, g2, b2) = crate::theme::hex_to_rgb(end_hex);
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len().max(1);
    let mut out = String::new();
    for (i, c) in chars.iter().enumerate() {
        let t = if n <= 1 {
            0.0
        } else {
            i as f32 / (n - 1) as f32
        };
        let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        out.push_str(&format!(
            "\x1b[38;2;{};{};{}m{}",
            mix(r1, r2),
            mix(g1, g2),
            mix(b1, b2),
            c
        ));
    }
    out
}

pub fn time_widget_strings() -> (String, String, String, f32) {
    let now = chrono::Local::now();
    let hm = now.format("%H:%M").to_string().to_lowercase();
    let date = now.format("%a %b %d").to_string().to_lowercase();
    let tz = now.format("%:z").to_string().to_lowercase();
    let secs_today = (now.format("%H").to_string().parse::<f32>().unwrap_or(0.0) * 3600.0)
        + (now.format("%M").to_string().parse::<f32>().unwrap_or(0.0) * 60.0)
        + now.format("%S").to_string().parse::<f32>().unwrap_or(0.0);
    let day_pct = (secs_today / 86400.0 * 100.0).clamp(0.0, 100.0);
    (hm, date, tz, day_pct)
}

pub fn visible_width(s: &str) -> usize {
    let mut in_escape = false;
    let mut width = 0;

    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' {
                in_escape = false;
            }
        } else {
            width += 1;
        }
    }

    width
}

pub fn pad_to_visible(s: &str, target_width: usize) -> String {
    let vis = visible_width(s);
    if target_width > vis {
        format!("{}{}", s, " ".repeat(target_width - vis))
    } else {
        s.to_string()
    }
}

// hard-truncate to `max` visible cols, passing ansi escapes through untouched.
// `format_line` runs every row through this, so a double-width nerd glyph in
// a foreign font (or a runaway value) eats trailing pad spaces instead of
// dislocating the right border. worst case the pad tints; the frame holds.
pub fn truncate_visible(s: &str, max: usize) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    let mut vis = 0;
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            out.push(c);
            for ec in chars.by_ref() {
                out.push(ec);
                if ec == 'm' {
                    break;
                }
            }
            continue;
        }
        if vis >= max {
            break;
        }
        out.push(c);
        vis += 1;
    }
    out
}

// accent color for a mascot color_role. exhaustive over every role
// git.rs can emit — no catch-all may silently dull a state again.
fn mascot_accent<'a>(color_role: &str, p: &'a Palette) -> &'a str {
    match color_role {
        "status_clean" => &p.status_clean,
        "status_dirty" => &p.status_dirty,
        "status_error" => &p.status_error,
        "task_active" => &p.task_active,
        "gauge_fill" => &p.gauge_fill,
        _ => &p.muted,
    }
}

// the 3-line cat plus status chip, every row exactly MASCOT_WIDTH visible
// cols. middle lines from git.rs are all 7 chars wide by construction;
// the chip is padded (longest label "all clear" lands exactly on 14).
fn mascot_rows(p: &Palette, mascot_color: &str, middle: &str, mood_word: &str) -> [String; 4] {
    let chip = format!("  {}● {}{}", mascot_color, mood_word, p.reset);
    [
        format!("   {}/\\_/\\{}      ", p.mascot, p.reset),
        format!("  {}{}{}     ", mascot_color, middle, p.reset),
        format!("   {}> ^ <{}      ", p.mascot, p.reset),
        pad_to_visible(&chip, MASCOT_WIDTH),
    ]
}

// Compact Default Mode (72 total width)
pub fn render_compact(git: &GitState, palette: &Palette, top_lang: Option<&str>) -> String {
    let p = palette;
    let mascot_state = compute_mascot_state(git, true);
    let visual = mascot_visual_for_state(mascot_state);

    let workspace_val = format_workspace_path(git.repo_root.as_deref());
    let git_pill = format_git_pill(git, p);
    let env_val = format_env_string(top_lang);

    let (hm, date_str, _tz, day_pct) = time_widget_strings();
    let time_val = format!(
        "{}{} • {}{}{}",
        gradient_text(&hm, &p.dot_workspace_hex, &p.title_mew_hex, p),
        p.reset,
        p.task_active,
        date_str,
        p.reset
    );
    let day_val = format!(
        "{} {}{}%{}",
        render_gauge_porcelain(day_pct, 8, &p.gauge_fill, &p.gauge_empty, &p.reset),
        p.gauge_fill,
        day_pct as usize,
        p.reset
    );

    let mascot_color = mascot_accent(visual.color_role, p);
    let [m1, m2, m3, m4] = mascot_rows(p, mascot_color, visual.middle_line, visual.mood_label);
    let m_blank = " ".repeat(MASCOT_WIDTH);

    let inner_width: usize = 68;
    let total_width: usize = inner_width + 4;

    let format_line = |left: &str, right: &str| -> String {
        let content = truncate_visible(&format!("{}{}", left, right), inner_width);
        let vis = visible_width(&content);
        let pad = inner_width.saturating_sub(vis);
        format!(
            "{}│{} {}{} {}│{}",
            p.border,
            p.reset,
            content,
            " ".repeat(pad),
            p.border,
            p.reset
        )
    };

    let empty_line = format!(
        "{}│{} {} {}│{}",
        p.border,
        p.reset,
        " ".repeat(inner_width),
        p.border,
        p.reset
    );

    let gutter = format!("{}│{}  ", p.border, p.reset);
    let os_sym = crate::system::os_icon();

    let row1_r = format!(
        "{}{}\u{f07b}{}  {}  {}{}{}",
        p.bold,
        p.dot_workspace,
        p.reset,
        label(p, "workspace"),
        p.primary,
        workspace_val,
        p.reset
    );
    let row2_r = if git.is_repo {
        format!(
            "{}\u{e725}{}  {}  {}{}{}",
            p.dot_git,
            p.reset,
            label(p, "git"),
            p.primary,
            git_pill,
            p.reset
        )
    } else {
        // outside any repo the card doubles as a fastfetch-style readout:
        // the git row becomes a live uptime row instead of dead text.
        let up = read_uptime()
            .unwrap_or_else(|| "—".to_string())
            .to_lowercase();
        format!(
            "{}\u{f017}{}  {}  {}{}{}",
            p.title_mew,
            p.reset,
            label(p, "uptime"),
            p.primary,
            up,
            p.reset
        )
    };
    let env_colored = match env_val.split_once(" • ") {
        Some((os, rest)) => format!(
            "{}{}{} • {}{}{}",
            p.dot_env, os, p.reset, p.primary, rest, p.reset
        ),
        None => format!("{}{}{}", p.primary, env_val, p.reset),
    };
    let row3_r = format!(
        "{}{}{}  {}  {}",
        p.dot_env,
        os_sym,
        p.reset,
        label(p, "env"),
        env_colored
    );
    let row4_r = format!(
        "{}\u{f017}{}  {}  {}",
        p.dot_task,
        p.reset,
        label(p, "time"),
        time_val
    );
    let row5_r = format!(
        "{}\u{f0e7}{}  {}  {}{}{}",
        p.dot_mood,
        p.reset,
        label(p, "day"),
        p.primary,
        day_val,
        p.reset
    );
    let row6_r = p.color_palette_dots();

    let mut out = String::new();
    out.push_str(&border_line(p, total_width, '╭', '╮', Some("mew")));
    out.push_str(&empty_line);
    out.push('\n');
    out.push_str(&format_line(&format!("{}{}", m1, gutter), &row1_r));
    out.push('\n');
    out.push_str(&format_line(&format!("{}{}", m2, gutter), &row2_r));
    out.push('\n');
    out.push_str(&format_line(&format!("{}{}", m3, gutter), &row3_r));
    out.push('\n');
    out.push_str(&format_line(&format!("{}{}", m4, gutter), &row4_r));
    out.push('\n');
    out.push_str(&format_line(&format!("{}{}", m_blank, gutter), &row5_r));
    out.push('\n');
    out.push_str(&empty_line);
    out.push('\n');
    out.push_str(&format_line(&format!("{}{}", m_blank, gutter), &row6_r));
    out.push('\n');
    out.push_str(&empty_line);
    out.push('\n');
    out.push_str(&border_line(p, total_width, '╰', '╯', None));

    out
}

use std::path::Path;

use git2::Repository;

use crate::git::{GitState, compute_mascot_state, mascot_visual_for_state};
use crate::system::{
    SystemTelemetry, read_os_name, read_rustc_version, read_toolchain_version, read_uptime,
};
use crate::telemetry::ProjectTelemetry;
use crate::theme::Palette;

const LABEL_WIDTH: usize = 9;
const SYS_LABEL_WIDTH: usize = 4;
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

fn sys_label(p: &Palette, text: &str) -> String {
    format!(
        "{}{:<width$}{}",
        p.muted,
        text,
        p.reset,
        width = SYS_LABEL_WIDTH
    )
}

fn border_line(
    p: &Palette,
    total_width: usize,
    left: char,
    right: char,
    label_text: Option<(&str, &str)>,
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
        Some((text, color)) => {
            let dashes = total_width.saturating_sub(5 + text.chars().count());
            format!(
                "{}{}─{} {}{}{}{} {}{}{}{}\n",
                p.border,
                left,
                p.reset,
                p.bold,
                color,
                text,
                p.reset,
                p.border,
                "─".repeat(dashes),
                right,
                p.reset
            )
        }
    }
}

// framed sub-card divider: ├─[ icon title ]──…──┤ with a derived dash run.
// visible math: ├(1) ─(1) [(1) sp(1) icon(1) sp(1) text(n) sp(1) ](1) +
// dashes + ┤(1), so dashes = total - 9 - n. icon is part of the frame
// chrome (single cell by contract), never a duplicate of row icons.
fn section_divider(p: &Palette, total_width: usize, icon: &str, text: &str, color: &str) -> String {
    let mut out = String::new();
    out.push_str(&p.border);
    out.push('├');
    out.push('─');
    out.push_str(color);
    out.push('[');
    out.push(' ');
    out.push_str(icon);
    out.push(' ');
    out.push_str(text);
    out.push(' ');
    out.push(']');
    out.push_str(&p.reset);
    out.push_str(&p.border);
    out.push_str(&"─".repeat(total_width.saturating_sub(9 + text.chars().count())));
    out.push('┤');
    out.push_str(&p.reset);
    out.push('\n');
    out
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

pub fn format_branch_info(git: &GitState) -> String {
    if !git.is_repo {
        return "not a git repository".to_string();
    }

    if git.is_detached {
        return format!("detached at head ({})", git.branch_name).to_lowercase();
    }

    if !git.ahead_behind.has_upstream {
        return "no upstream configured".to_string();
    }

    let upstream = git
        .ahead_behind
        .upstream_name
        .as_deref()
        .unwrap_or("upstream");

    match (git.ahead_behind.ahead, git.ahead_behind.behind) {
        (0, 0) => format!("up to date with {}", upstream).to_lowercase(),
        (a, 0) => format!("{} ahead of {}", a, upstream).to_lowercase(),
        (0, b) => format!("{} behind {}", b, upstream).to_lowercase(),
        (a, b) => format!("{} ahead, {} behind {}", a, b, upstream).to_lowercase(),
    }
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

// latest commit subject for the telemetry grid. honest "—" when unknown
// (empty repo, no git dir, unreadable object). never fabricated.
fn head_commit_summary(repo_root: Option<&Path>) -> String {
    let root = match repo_root {
        Some(r) => r,
        None => return "—".to_string(),
    };
    let repo = match Repository::open(root) {
        Ok(r) => r,
        Err(_) => return "—".to_string(),
    };
    let oid = match repo.head().ok().and_then(|h| h.target()) {
        Some(o) => o,
        None => return "—".to_string(),
    };
    match repo.find_commit(oid) {
        Ok(c) => match c.summary() {
            Ok(Some(s)) => s.to_lowercase(),
            _ => "—".to_string(),
        },
        Err(_) => "—".to_string(),
    }
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
    out.push_str(&border_line(
        p,
        total_width,
        '╭',
        '╮',
        Some(("mew", &p.title_mew)),
    ));
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

fn sys_cell(
    p: &Palette,
    icon: &str,
    accent: &str,
    lbl: &str,
    meter: String,
    pct: String,
    detail: String,
) -> String {
    format!(
        "  {}{}{}  {} {}  {}  {}",
        accent,
        icon,
        p.reset,
        sys_label(p, lbl),
        meter,
        pct,
        detail
    )
}

// Full Mode (--full, 79 total width)
pub fn render_full(
    force_count_lines: bool,
    git: &GitState,
    sys: &SystemTelemetry,
    telem: &ProjectTelemetry,
    palette: &Palette,
) -> String {
    let p = palette;
    let mascot_state = compute_mascot_state(git, true);
    let visual = mascot_visual_for_state(mascot_state);

    let workspace_val = format_workspace_path(git.repo_root.as_deref());
    let git_pill = format_git_pill(git, p);
    let env_val = format_env_string(telem.top_language.as_ref().map(|(lang, _)| lang.as_str()));

    let mascot_color = mascot_accent(visual.color_role, p);
    let [m1, m2, m3, m4] = mascot_rows(p, mascot_color, visual.middle_line, visual.mood_label);

    let inner_width: usize = 75;
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
    let row2_r = format!(
        "{}\u{e725}{}  {}  {}[ {}{}]{}",
        p.dot_git,
        p.reset,
        label(p, "git"),
        p.primary,
        git_pill,
        p.primary,
        p.reset
    );
    let row3_r = format!(
        "{}{}{}  {}  {}{}{}",
        p.dot_env,
        os_sym,
        p.reset,
        label(p, "env"),
        p.primary,
        env_val,
        p.reset
    );
    let row4_r = p.color_palette_dots();

    let cpu_accent = &p.gauge_fill; // Sapphire / Blue (#89b4fa)
    let ram_accent = &p.status_clean; // Green (#a6e3a1)
    let disk_accent = &p.dot_git; // Peach (#fab387)
    let up_accent = &p.title_mew; // Mauve (#cba6f7)

    let smooth_or_dash = |pct: Option<f32>, accent: &str| -> String {
        match pct {
            Some(v) => render_gauge_porcelain(v, 10, accent, &p.gauge_empty, &p.reset),
            None => format!("{}{:<10}{}", p.muted, "─", p.reset),
        }
    };
    let tinted_pct = |pct: Option<f32>, accent: &str| -> String {
        match pct {
            Some(v) => format!("{}{}{:<4}{}", p.bold, accent, format!("{:.0}%", v), p.reset),
            None => format!("{:<4}", ""),
        }
    };
    let accent_detail =
        |text: String, accent: &str| -> String { format!("{}{:<8}{}", accent, text, p.reset) };

    let cpu_freq_str = match sys.cpu_freq_ghz {
        Some(f) => format!("{:.1}ghz", f),
        None => String::new(),
    };
    let cpu_part = sys_cell(
        p,
        "\u{f4bc}",
        cpu_accent,
        "cpu",
        smooth_or_dash(sys.cpu_usage, cpu_accent),
        tinted_pct(sys.cpu_usage, cpu_accent),
        accent_detail(cpu_freq_str, cpu_accent),
    );

    let ram_val_str = match (sys.ram_used_gb, sys.ram_total_gb) {
        (Some(u), Some(t)) => format!("{:.1}/{:.0}gb", u, t),
        _ => String::new(),
    };
    let ram_part = sys_cell(
        p,
        "\u{e266}",
        ram_accent,
        "ram",
        smooth_or_dash(sys.ram_percent, ram_accent),
        tinted_pct(sys.ram_percent, ram_accent),
        accent_detail(ram_val_str, ram_accent),
    );

    let disk_val_str = match (sys.disk_used_gb, sys.disk_total_gb) {
        (Some(u), Some(t)) => format!("{:.0}/{:.0}gb", u, t),
        _ => String::new(),
    };
    let disk_part = sys_cell(
        p,
        "\u{f02ca}",
        disk_accent,
        "disk",
        smooth_or_dash(sys.disk_percent, disk_accent),
        tinted_pct(sys.disk_percent, disk_accent),
        accent_detail(disk_val_str, disk_accent),
    );

    let up_val_str = match &sys.uptime_formatted {
        Some(u) => format!("[{}]", u.to_lowercase()),
        None => "—".to_string(),
    };
    let up_meter = if sys.uptime_formatted.is_some() {
        format!("{}{:<14}{}", p.primary, up_val_str, p.reset)
    } else {
        format!("{}{:<14}{}", p.muted, up_val_str, p.reset)
    };
    let up_detail = match &sys.uptime_formatted {
        Some(_) => format!("{}{:<8}{}", p.status_clean, "stable", p.reset),
        None => format!("{}{:<8}{}", p.muted, "—", p.reset),
    };
    let up_part = sys_cell(
        p,
        "\u{f017}",
        up_accent,
        "up",
        up_meter,
        String::new(),
        up_detail,
    );

    let sys_line1 = format!("{}   {}", cpu_part, ram_part);
    let sys_line2 = format!("{}   {}", disk_part, up_part);

    let (hm_full, date_full, tz_full, day_pct_full) = time_widget_strings();
    let time_hdr = format!("  {}time{}", p.muted, p.reset);
    let ctx_hdr = format!("{}context{}", p.muted, p.reset);
    let ctx_header_line = format!("{}  {}", pad_to_visible(&time_hdr, 36), ctx_hdr);

    fn truncate_str(s: &str, max_len: usize) -> String {
        if s.chars().count() <= max_len {
            s.to_string().to_lowercase()
        } else {
            let mut res: String = s.chars().take(max_len.saturating_sub(2)).collect();
            res.push_str("..");
            res.to_lowercase()
        }
    }

    let clock_big = format!(
        "  {}\u{f017}{}  {}  {}{}",
        p.dot_mood,
        p.reset,
        label(p, "clock"),
        gradient_text(&hm_full, &p.dot_workspace_hex, &p.title_mew_hex, p),
        p.reset
    );
    let date_line = format!(
        "  {}\u{f455}{}  {}  {}{} • {}{}{}",
        p.dot_mood,
        p.reset,
        label(p, "date"),
        p.primary,
        date_full,
        p.muted,
        tz_full,
        p.reset
    );

    // Fallback handling for HEAD: honest states only, never a plausible fake hash
    let head_val = if git.commit_hash.trim().is_empty() || git.commit_hash == "—" {
        if git.is_detached {
            "detached".to_string()
        } else {
            "uncommitted".to_string()
        }
    } else {
        git.commit_hash.clone()
    };

    let head_str = format!(
        "{}\u{e725}{}  {}  {}{}{}",
        p.status_clean,
        p.reset,
        label(p, "head"),
        p.status_clean,
        truncate_str(&head_val, 22),
        p.reset
    );
    let remote_str = format!(
        "{}\u{f019f}{}  {}  {}{}{}",
        p.gauge_fill,
        p.reset,
        label(p, "remote"),
        p.primary,
        truncate_str(&format_branch_info(git), 22),
        p.reset
    );
    let day_mini = format!(
        "  {}\u{f0e7}{}  {}  {} {}{}%{}",
        p.gauge_fill,
        p.reset,
        label(p, "day"),
        render_gauge_porcelain(day_pct_full, 8, &p.gauge_fill, &p.gauge_empty, &p.reset),
        p.gauge_fill,
        day_pct_full as usize,
        p.reset
    );
    let (shell_name, term_name) = crate::system::read_shell_info();
    let session_val = truncate_str(&format!("{} • {}", shell_name, term_name), 22);
    let shell_str = format!(
        "{}\u{276f}{}  {}  {}{}{}",
        p.title_cmd,
        p.reset,
        label(p, "shell"),
        p.primary,
        session_val,
        p.reset
    );

    let ctx_row1 = format!("{}  {}", pad_to_visible(&clock_big, 36), head_str);
    let ctx_row2 = format!("{}  {}", pad_to_visible(&date_line, 36), remote_str);
    let ctx_row3 = format!("{}  {}", pad_to_visible(&day_mini, 36), shell_str);

    // grid math: left cell 38 (16 prefix + 22 value) + 2 gutter +
    // right cell 35 (16 prefix + 19 value) = 75. every variable value is
    // truncated into its slot, so long counts/names can never overflow.
    let loc_val = if git.is_repo || force_count_lines {
        pad_to_visible(&format!("{} lines", telem.total_lines), 22)
    } else {
        pad_to_visible("line count unavailable", 22)
    };

    let loc_cell = format!(
        "  {}\u{f121}{}  {}  {}{}{}",
        p.gauge_fill,
        p.reset,
        label(p, "loc"),
        p.primary,
        loc_val,
        p.reset
    );

    let lang_val = if telem.total_lines == 0 {
        pad_to_visible(&format!("{}{}{}", p.muted, "—", p.reset), 22)
    } else {
        let (lang_name, lang_pct) = telem
            .top_language
            .clone()
            .unwrap_or_else(|| ("—".to_string(), 0.0));
        // prefer full language names over decimal precision in the 13-col
        // slot: "100% gdscript" fits, "100.0% gdsc.." does not.
        let mut lang_label = format!("{:.1}% {}", lang_pct, lang_name);
        if lang_label.chars().count() > 13 {
            let short = format!("{:.0}% {}", lang_pct, lang_name);
            if short.chars().count() <= 13 {
                lang_label = short;
            }
        }
        let lang_text = format!(
            "{}{}{}",
            p.gauge_fill,
            truncate_str(&lang_label, 13),
            p.reset
        );
        pad_to_visible(
            &format!(
                "{} {}",
                render_gauge_porcelain(lang_pct, 8, &p.gauge_fill, &p.gauge_empty, &p.reset),
                lang_text
            ),
            22,
        )
    };
    let lang_cell = format!(
        "  {}\u{f410}{}  {}  {}",
        p.title_cmd,
        p.reset,
        label(p, "lang"),
        lang_val
    );

    // coverage is shown only from a real report file; a missing report
    // becomes a muted pill badge instead of a bare dash (zero dead space).
    let cov_raw = telem.coverage_info.trim();
    let cov_missing = cov_raw.is_empty() || cov_raw.contains("no coverage");
    let cov_inner = if cov_missing {
        pad_to_visible(&format!("{}[ untracked ]{}", p.muted, p.reset), 19)
    } else {
        pad_to_visible(
            &format!("{}{}{}", p.gauge_fill, truncate_str(cov_raw, 19), p.reset),
            19,
        )
    };
    let cov_icon = if cov_missing { &p.muted } else { &p.gauge_fill };
    let cov_cell = format!(
        "  {}\u{f0c3}{}  {}  {}",
        cov_icon,
        p.reset,
        label(p, "coverage"),
        cov_inner
    );

    // latest commit subject, read live from git. nothing to show becomes
    // a muted pill badge naming the honest state (never a bare dash).
    let commit_summary = head_commit_summary(git.repo_root.as_deref());
    let commit_inner = if commit_summary == "—" {
        let tag = if git.is_repo {
            "uncommitted"
        } else {
            "no repo"
        };
        pad_to_visible(&format!("{}[ {} ]{}", p.muted, tag, p.reset), 19)
    } else {
        pad_to_visible(
            &format!(
                "{}{}{}",
                p.primary,
                truncate_str(&commit_summary, 19),
                p.reset
            ),
            19,
        )
    };
    let commit_icon = if commit_summary == "—" {
        &p.muted
    } else {
        &p.title_mew
    };
    let commit_cell = format!(
        "  {}\u{e729}{}  {}  {}",
        commit_icon,
        p.reset,
        label(p, "commit"),
        commit_inner
    );

    let telem_row1 = format!("{}  {}", loc_cell, cov_cell);
    let telem_row2 = format!("{}  {}", lang_cell, commit_cell);

    let mut out = String::new();

    out.push_str(&border_line(
        p,
        total_width,
        '╭',
        '╮',
        Some(("mew", &p.title_mew)),
    ));
    out.push_str(&empty_line);
    out.push('\n');
    out.push_str(&format_line(&format!("{}{}", m1, gutter), &row1_r));
    out.push('\n');
    out.push_str(&format_line(&format!("{}{}", m2, gutter), &row2_r));
    out.push('\n');
    out.push_str(&format_line(&format!("{}{}", m3, gutter), &row3_r));
    out.push('\n');
    out.push_str(&empty_line);
    out.push('\n');
    out.push_str(&format_line(&format!("{}{}", m4, gutter), &row4_r));
    out.push('\n');
    out.push_str(&empty_line);
    out.push('\n');

    out.push_str(&section_divider(
        p,
        total_width,
        "\u{f4bc}",
        "system & health",
        &p.title_sys,
    ));
    out.push_str(&empty_line);
    out.push('\n');
    out.push_str(&format_line("", &sys_line1));
    out.push('\n');
    out.push_str(&format_line("", &sys_line2));
    out.push('\n');
    out.push_str(&empty_line);
    out.push('\n');

    out.push_str(&section_divider(
        p,
        total_width,
        "\u{f017}",
        "active context",
        &p.title_context,
    ));
    out.push_str(&empty_line);
    out.push('\n');
    out.push_str(&format_line("", &ctx_header_line));
    out.push('\n');
    out.push_str(&format_line("", &ctx_row1));
    out.push('\n');
    out.push_str(&format_line("", &ctx_row2));
    out.push('\n');
    out.push_str(&format_line("", &ctx_row3));
    out.push('\n');
    out.push_str(&empty_line);
    out.push('\n');

    out.push_str(&section_divider(
        p,
        total_width,
        "\u{f121}",
        "project telemetry",
        &p.title_telem,
    ));
    out.push_str(&empty_line);
    out.push('\n');
    out.push_str(&format_line("", &telem_row1));
    out.push('\n');
    out.push_str(&format_line("", &telem_row2));
    out.push('\n');
    out.push_str(&empty_line);
    out.push('\n');

    out.push_str(&border_line(p, total_width, '╰', '╯', None));

    out
}

use clap::error::ErrorKind;

use crate::render::gradient_text;
use crate::theme::Palette;

// which surface produced clap's output, so line 0 gets the brand treatment:
// help gets the wordmark next to the about line, version gets wordmark +
// bold number, errors stay terse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClapScreen {
    Help,
    Version,
    Error,
}

pub fn classify(kind: &ErrorKind) -> ClapScreen {
    match kind {
        ErrorKind::DisplayHelp | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            ClapScreen::Help
        }
        ErrorKind::DisplayVersion => ClapScreen::Version,
        _ => ClapScreen::Error,
    }
}

// clap's auto-generated strings, mapped to the casing style this project
// uses. matched exactly and nothing else — no fuzzy rewrites.
fn normalize_generated_text(desc: &str) -> &str {
    match desc {
        "Print help" => "print help",
        "Print version" => "print version",
        "Print this message or the help of the given subcommand(s)" => {
            "print this message or the help of the given subcommand(s)"
        }
        other => other,
    }
}

fn push_span(out: &mut String, text: &str, color: &str, p: &Palette) {
    out.push_str(color);
    out.push_str(text);
    out.push_str(&p.reset);
}

// color single-quoted spans ('hook', '--help') with an accent: a quoted
// thing in clap output is always the thing the user is meant to act on.
fn push_quoted(out: &mut String, text: &str, base: &str, accent: &str, p: &Palette) {
    let mut rest = text;
    while let Some(start) = rest.find('\'') {
        match rest[start + 1..].find('\'') {
            Some(rel_end) => {
                let end = start + 1 + rel_end;
                push_span(out, &rest[..start + 1], base, p); // leading text + opening quote
                push_span(out, &rest[start + 1..=end], accent, p); // inner text + closing quote
                rest = &rest[end + 1..];
            }
            None => break,
        }
    }
    if !rest.is_empty() {
        push_span(out, rest, base, p);
    }
}

// usage-style line: the wordmark is painted with the brand gradient,
// <angle> and [square] placeholders in task_active, flags in dot_env.
fn push_usage(out: &mut String, rest: &str, p: &Palette) {
    for (i, token) in rest.split(' ').enumerate() {
        if i > 0 {
            out.push(' ');
        }
        if token == "mew" {
            out.push_str(&gradient_text(
                "mew",
                &p.dot_workspace_hex,
                &p.title_mew_hex,
                p,
            ));
            out.push_str(&p.reset);
        } else if token.starts_with('<') || token.starts_with('[') {
            push_span(out, token, &p.task_active, p);
        } else if token.starts_with('-') {
            push_span(out, token, &p.dot_env, p);
        } else {
            push_span(out, token, &p.primary, p);
        }
    }
}

// entry lines ("  theme   print the default ..."): names in dot_workspace,
// flags in dot_env, placeholders in task_active, description in primary.
// clap separates the column with a run of two or more spaces.
fn push_entry(out: &mut String, indent: &str, trimmed: &str, p: &Palette) {
    let Some(split) = trimmed.find("  ") else {
        push_span(out, trimmed, &p.primary, p);
        return;
    };
    let (names, desc) = trimmed.split_at(split);
    let desc = normalize_generated_text(desc.trim());
    out.push_str(indent);
    for (i, name) in names.split(", ").enumerate() {
        if i > 0 {
            out.push_str(&p.muted);
            out.push_str(", ");
            out.push_str(&p.reset);
        }
        let color = if name.starts_with('-') {
            &p.dot_env
        } else if name.starts_with('<') {
            &p.task_active
        } else {
            &p.dot_workspace
        };
        push_span(out, name, color, p);
    }
    out.push_str("  ");
    push_span(out, desc, &p.primary, p);
}

fn style_line(line: &str, first: bool, screen: ClapScreen, p: &Palette) -> String {
    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];
    let mut out = String::with_capacity(line.len() + 64);

    if first && screen == ClapScreen::Help && !trimmed.is_empty() && !trimmed.starts_with("error:")
    {
        out.push_str(&p.bold);
        out.push_str(&gradient_text(
            "mew",
            &p.dot_workspace_hex,
            &p.title_mew_hex,
            p,
        ));
        out.push_str(&p.reset);
        out.push_str(&p.muted);
        out.push_str(" · ");
        out.push_str(&p.reset);
        push_span(&mut out, trimmed, &p.primary, p);
        return out;
    }
    if first && screen == ClapScreen::Version {
        let rest = trimmed.strip_prefix("mew ").unwrap_or(trimmed);
        out.push_str(&p.bold);
        out.push_str(&gradient_text(
            "mew",
            &p.dot_workspace_hex,
            &p.title_mew_hex,
            p,
        ));
        out.push_str(&p.reset);
        out.push(' ');
        push_span(&mut out, rest, &p.primary, p);
        return out;
    }
    if let Some(rest) = trimmed.strip_prefix("error: ") {
        out.push_str(&p.bold);
        push_span(&mut out, "error:", &p.status_error, p);
        out.push(' ');
        push_quoted(&mut out, rest, &p.primary, &p.dot_workspace, p);
        return out;
    }
    if let Some(rest) = trimmed.strip_prefix("warning: ") {
        out.push_str(&p.bold);
        push_span(&mut out, "warning:", &p.status_dirty, p);
        out.push(' ');
        push_quoted(&mut out, rest, &p.primary, &p.dot_workspace, p);
        return out;
    }
    if let Some(rest) = trimmed.strip_prefix("tip: ") {
        out.push_str(indent);
        out.push_str(&p.bold);
        push_span(&mut out, "tip:", &p.task_active, p);
        out.push(' ');
        push_quoted(&mut out, rest, &p.primary, &p.dot_workspace, p);
        return out;
    }
    if let Some(rest) = trimmed.strip_prefix("Usage: ") {
        out.push_str(indent);
        out.push_str(&p.bold);
        push_span(&mut out, "usage:", &p.muted, p);
        out.push(' ');
        push_usage(&mut out, rest, p);
        return out;
    }
    if trimmed.starts_with("For more information") {
        out.push_str(indent);
        push_span(&mut out, "try", &p.muted, p);
        out.push(' ');
        push_span(&mut out, "--help", &p.dot_env, p);
        push_span(&mut out, " for more information", &p.muted, p);
        return out;
    }
    match trimmed {
        "Commands:" => {
            out.push_str(&p.bold);
            push_span(&mut out, "commands:", &p.title_mew, p);
        }
        "Arguments:" => {
            out.push_str(&p.bold);
            push_span(&mut out, "arguments:", &p.title_mew, p);
        }
        "Options:" => {
            out.push_str(&p.bold);
            push_span(&mut out, "options:", &p.title_mew, p);
        }
        "Subcommands:" => {
            out.push_str(&p.bold);
            push_span(&mut out, "subcommands:", &p.title_mew, p);
        }
        "Global options:" => {
            out.push_str(&p.bold);
            push_span(&mut out, "global options:", &p.title_mew, p);
        }
        _ => {
            if trimmed.starts_with('<') {
                out.push_str(indent);
                push_span(&mut out, trimmed, &p.task_active, p);
            } else if trimmed.find("  ").is_some() {
                push_entry(&mut out, indent, trimmed, p);
            } else if trimmed.is_empty() {
                return String::new();
            } else {
                out.push_str(indent);
                push_span(&mut out, trimmed, &p.primary, p);
            }
        }
    }
    out
}

// restyle clap's rendered error/help output into the mew palette. plain
// mode applies the same text without escape bytes, so piped output stays
// byte-clean and script-safe.
pub fn style_clap_output(src: &str, screen: ClapScreen, p: &Palette) -> String {
    let mut out = String::with_capacity(src.len() + 512);
    for (i, line) in src.lines().enumerate() {
        out.push_str(&style_line(line, i == 0, screen, p));
        out.push('\n');
    }
    out
}

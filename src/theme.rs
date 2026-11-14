use std::env;
use std::fs;
use std::io::IsTerminal;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// container default: old theme.toml files that lack the newer accent keys
// keep parsing, missing keys fall back to the palette below.
#[serde(default)]
pub struct ThemeColors {
    pub border: String,
    pub primary: String,
    pub muted: String,
    pub mascot: String,
    pub task_active: String,
    pub gauge_fill: String,
    pub gauge_empty: String,
    pub status_clean: String,
    pub status_dirty: String,
    pub status_error: String,
    // per-item accents: row icons and the wordmark gradient stops
    pub dot_workspace: String,
    pub dot_git: String,
    pub dot_env: String,
    pub dot_task: String,
    pub dot_mood: String,
    pub title_mew: String,
    // signature fastfetch palette strip
    pub dots: Vec<String>,
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            border: "#45475a".to_string(),
            primary: "#cdd6f4".to_string(),
            muted: "#6c7086".to_string(),
            mascot: "#f2cdcd".to_string(),
            task_active: "#b4befe".to_string(),
            gauge_fill: "#89b4fa".to_string(),
            gauge_empty: "#313244".to_string(),
            status_clean: "#a6e3a1".to_string(),
            status_dirty: "#f9e2af".to_string(),
            status_error: "#f38ba8".to_string(),
            dot_workspace: "#89dceb".to_string(),
            dot_git: "#fab387".to_string(),
            dot_env: "#cba6f7".to_string(),
            dot_task: "#a6e3a1".to_string(),
            dot_mood: "#f5c2e7".to_string(),
            title_mew: "#cba6f7".to_string(),
            dots: vec![
                "#f38ba8".to_string(), // red
                "#fab387".to_string(), // peach
                "#f9e2af".to_string(), // yellow
                "#a6e3a1".to_string(), // green
                "#94e2d5".to_string(), // teal
                "#89b4fa".to_string(), // blue
                "#cba6f7".to_string(), // mauve
                "#f5c2e7".to_string(), // pink
            ],
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub colors: ThemeColors,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Truecolor,
    Plain,
}

pub fn detect_color_mode() -> ColorMode {
    // strip ansi if stdout is not a tty
    if !std::io::stdout().is_terminal() {
        return ColorMode::Plain;
    }

    // respect no_color standard (https://no-color.org)
    if env::var_os("NO_COLOR").is_some() {
        return ColorMode::Plain;
    }

    // truecolor detection via colorterm or 24bit terms
    if let Ok(colorterm) = env::var("COLORTERM")
        && (colorterm == "truecolor" || colorterm == "24bit")
    {
        return ColorMode::Truecolor;
    }

    if let Ok(term) = env::var("TERM")
        && (term.contains("24bit") || term.contains("truecolor") || term.contains("xterm-256color"))
    {
        return ColorMode::Truecolor;
    }

    ColorMode::Truecolor
}

// parses #rrggbb hex string to (r, g, b) tuple
pub fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() != 6 {
        return (255, 255, 255);
    }
    let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(255);
    let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(255);
    let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(255);
    (r, g, b)
}

// converts #rrggbb hex string to 24-bit truecolor escape sequence
pub fn hex_to_ansi(hex: &str) -> String {
    let (r, g, b) = hex_to_rgb(hex);
    format!("\x1b[38;2;{};{};{}m", r, g, b)
}

pub fn theme_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("mew"))
}

pub fn theme_path() -> Option<PathBuf> {
    theme_dir().map(|p| p.join("theme.toml"))
}

pub fn load_theme() -> ThemeConfig {
    if let Some(path) = theme_path()
        && path.exists()
    {
        match fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<ThemeConfig>(&content) {
                Ok(theme) => return theme,
                Err(err) => {
                    eprintln!("mew: warning: failed to parse {}: {}", path.display(), err);
                }
            },
            Err(err) => {
                eprintln!("mew: warning: failed to read {}: {}", path.display(), err);
            }
        }
    }

    ThemeConfig::default()
}

// ansi-escape palette for static rendering
pub struct Palette {
    pub border: String,
    pub primary: String,
    pub muted: String,
    pub mascot: String,
    pub task_active: String,
    pub gauge_fill: String,
    pub gauge_empty: String,
    pub status_clean: String,
    pub status_dirty: String,
    pub status_error: String,
    // zap-inspired accent dot colors
    pub dot_workspace: String,
    pub dot_git: String,
    pub dot_env: String,
    pub dot_task: String,
    pub dot_mood: String,
    // signature accent for the wordmark gradient and chrome highlights
    pub title_mew: String,
    // raw hex stops for the clock gradient (ansi escapes can't unmix)
    pub dot_workspace_hex: String,
    pub title_mew_hex: String,
    // palette strip, straight from theme.toml
    pub dots: Vec<String>,
    pub reset: String,
    pub is_plain: bool,
    // single emphasis tier (linear 510/590 equivalent): bold titles and key
    // numbers. empty in plain mode so piped output stays clean by construction.
    pub bold: String,
}

impl Palette {
    pub fn from_config(config: &ThemeConfig, mode: ColorMode) -> Self {
        if mode == ColorMode::Plain {
            return Self {
                border: String::new(),
                primary: String::new(),
                muted: String::new(),
                mascot: String::new(),
                task_active: String::new(),
                gauge_fill: String::new(),
                gauge_empty: String::new(),
                status_clean: String::new(),
                status_dirty: String::new(),
                status_error: String::new(),
                dot_workspace: String::new(),
                dot_git: String::new(),
                dot_env: String::new(),
                dot_task: String::new(),
                dot_mood: String::new(),
                title_mew: String::new(),
                dot_workspace_hex: String::new(),
                title_mew_hex: String::new(),
                dots: Vec::new(),
                reset: String::new(),
                is_plain: true,
                bold: String::new(),
            };
        }

        let c = &config.colors;
        Self {
            border: hex_to_ansi(&c.border),
            primary: hex_to_ansi(&c.primary),
            muted: hex_to_ansi(&c.muted),
            mascot: hex_to_ansi(&c.mascot),
            task_active: hex_to_ansi(&c.task_active),
            gauge_fill: hex_to_ansi(&c.gauge_fill),
            gauge_empty: hex_to_ansi(&c.gauge_empty),
            status_clean: hex_to_ansi(&c.status_clean),
            status_dirty: hex_to_ansi(&c.status_dirty),
            status_error: hex_to_ansi(&c.status_error),
            // per-item accents, all theme-driven (no hardcoded hex here)
            dot_workspace: hex_to_ansi(&c.dot_workspace),
            dot_git: hex_to_ansi(&c.dot_git),
            dot_env: hex_to_ansi(&c.dot_env),
            dot_task: hex_to_ansi(&c.dot_task),
            dot_mood: hex_to_ansi(&c.dot_mood),
            // colorful distinct card headers
            title_mew: hex_to_ansi(&c.title_mew),
            dot_workspace_hex: c.dot_workspace.clone(),
            title_mew_hex: c.title_mew.clone(),
            dots: c.dots.clone(),
            reset: "\x1b[0m".to_string(),
            is_plain: false,
            bold: "\x1b[1m".to_string(),
        }
    }

    // signature fastfetch / unixporn 8-color palette test dots, from theme
    pub fn color_palette_dots(&self) -> String {
        if self.is_plain {
            return "● ● ● ● ● ● ● ●".to_string();
        }
        let mut out = String::new();
        for (i, hex) in self.dots.iter().enumerate() {
            if i > 0 {
                out.push(' ');
            }
            out.push_str(&hex_to_ansi(hex));
            out.push('●');
            out.push_str(&self.reset);
        }
        out
    }
}

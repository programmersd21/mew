#[cfg(target_os = "linux")]
use std::fs;

#[cfg(target_os = "linux")]
pub fn read_uptime() -> Option<String> {
    let content = fs::read_to_string("/proc/uptime").ok()?;
    let first = content.split_whitespace().next()?;
    let seconds_float: f64 = first.parse().ok()?;
    let total_secs = seconds_float as u64;

    let days = total_secs / 86400;
    let rem = total_secs % 86400;
    let hours = rem / 3600;
    let rem = rem % 3600;
    let mins = rem / 60;

    if days > 0 {
        Some(format!("{}d {}h {}m", days, hours, mins))
    } else if hours > 0 {
        Some(format!("{}h {}m", hours, mins))
    } else {
        Some(format!("{}m", mins))
    }
}

#[cfg(not(target_os = "linux"))]
pub fn read_uptime() -> Option<String> {
    let total_secs = sysinfo::System::uptime();
    if total_secs == 0 {
        return Some("0m".to_string());
    }
    let days = total_secs / 86400;
    let rem = total_secs % 86400;
    let hours = rem / 3600;
    let rem = rem % 3600;
    let mins = rem / 60;
    if days > 0 {
        Some(format!("{}d {}h {}m", days, hours, mins))
    } else if hours > 0 {
        Some(format!("{}h {}m", hours, mins))
    } else {
        Some(format!("{}m", mins))
    }
}

// os name (from /etc/os-release PRETTY_NAME or NAME, falling back to std::env::consts::OS)
#[cfg(target_os = "linux")]
pub fn read_os_name() -> String {
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if line.starts_with("PRETTY_NAME=") {
                let val = line.trim_start_matches("PRETTY_NAME=").trim_matches('"');
                return val.to_lowercase();
            }
        }
        for line in content.lines() {
            if line.starts_with("NAME=") {
                let val = line.trim_start_matches("NAME=").trim_matches('"');
                return val.to_lowercase();
            }
        }
    }

    std::env::consts::OS.to_lowercase()
}

#[cfg(not(target_os = "linux"))]
pub fn read_os_name() -> String {
    if let Some(long) = sysinfo::System::long_os_version()
        && !long.trim().is_empty()
    {
        return long.to_lowercase();
    }
    if let Some(name) = sysinfo::System::name() {
        if let Some(ver) = sysinfo::System::os_version()
            && !ver.trim().is_empty()
        {
            return format!("{} {}", name, ver).to_lowercase();
        }
        if !name.trim().is_empty() {
            return name.to_lowercase();
        }
    }
    std::env::consts::OS.to_lowercase()
}

// distro-specific nerd font icon matching fastfetch / starship
pub fn os_icon() -> &'static str {
    let os = read_os_name();
    if os.contains("cachy") || os.contains("arch") {
        "\u{f303}" // nf-linux-archlinux
    } else if os.contains("ubuntu") {
        "\u{f31b}" // nf-linux-ubuntu
    } else if os.contains("fedora") {
        "\u{f30a}" // nf-linux-fedora
    } else if os.contains("debian") {
        "\u{f306}" // nf-linux-debian
    } else if os.contains("nix") {
        "\u{f313}" // nf-linux-nixos
    } else if os.contains("darwin") || os.contains("mac") {
        "\u{f179}" // nf-fa-apple
    } else if os.contains("windows") {
        "\u{f17a}" // nf-fa-windows
    } else {
        "\u{e712}" // nf-dev-linux
    }
}

// rustc version discovery (one subprocess per process at most: the binary
// renders once and exits, so there is nothing worth memoizing here)
pub fn read_rustc_version() -> Option<String> {
    let output = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    // e.g. "rustc 1.85.0 (4d91de4e4 2025-02-17)" -> "rustc 1.85.0"
    // (suffixes like "-nightly" are stripped for a stable width)
    let words: Vec<&str> = stdout.split_whitespace().collect();
    if words.len() >= 2 {
        let ver: String = words[1]
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        if !ver.is_empty() {
            return Some(format!("rustc {}", ver.to_lowercase()));
        }
    }
    None
}

// language-aware toolchain probe for the env row: runs exactly one
// subprocess for the detected project language (same budget as the old
// always-rustc call). kotlin is deliberately excluded: kotlinc needs a
// jvm boot measured in seconds, far outside the latency budget.
pub fn read_toolchain_version(lang: &str) -> Option<String> {
    const VERSION_FLAG: &[&str] = &["--version"];
    const VERSION_CMD: &[&str] = &["version"];
    const V_FLAG: &[&str] = &["-v"];

    let key = lang.to_lowercase();
    if key == "kotlin" {
        return None;
    }
    if key == "shell" {
        return shell_toolchain();
    }
    let (name, cmd, args): (Option<&str>, &str, &[&str]) = match key.as_str() {
        "rust" => (Some("rustc"), "rustc", VERSION_FLAG),
        "python" => (Some("python"), "python3", VERSION_FLAG),
        "go" => (Some("go"), "go", VERSION_CMD),
        "javascript" | "typescript" | "react" | "vue" | "svelte" => {
            (Some("node"), "node", VERSION_FLAG)
        }
        "c" => (None, "cc", VERSION_FLAG),
        "c++" => (None, "c++", VERSION_FLAG),
        "c#" | "csharp" => (Some("dotnet"), "dotnet", VERSION_FLAG),
        "gdscript" | "godot" | "gdshader" => (Some("godot"), "godot", VERSION_FLAG),
        "dart" => (Some("dart"), "dart", VERSION_FLAG),
        "java" | "groovy" => (None, "java", VERSION_FLAG),
        "swift" => (Some("swift"), "swift", VERSION_FLAG),
        "ruby" => (Some("ruby"), "ruby", VERSION_FLAG),
        "php" => (Some("php"), "php", VERSION_FLAG),
        "zig" => (Some("zig"), "zig", VERSION_CMD),
        "lua" => (Some("lua"), "lua", V_FLAG),
        "haskell" => (Some("ghc"), "ghc", VERSION_FLAG),
        "elixir" => (Some("elixir"), "elixir", VERSION_FLAG),
        "erlang" => (Some("erl"), "erl", &["-version"]),
        "clojure" => (Some("clojure"), "clojure", VERSION_FLAG),
        "scala" => (Some("scala"), "scala", VERSION_FLAG),
        "r" => (Some("r"), "R", VERSION_FLAG),
        "julia" => (Some("julia"), "julia", VERSION_FLAG),
        "perl" => (Some("perl"), "perl", V_FLAG),
        "terraform" => (Some("terraform"), "terraform", VERSION_FLAG),
        "powershell" => (Some("pwsh"), "pwsh", VERSION_FLAG),
        _ => return None,
    };

    let line = probe_version(cmd, args).or_else(|| {
        if key == "python" {
            probe_version("python", args)
        } else if key == "c" {
            probe_version("gcc", args)
                .or_else(|| probe_version("clang", args))
                .or_else(|| probe_version("cl", args))
        } else if key == "c++" {
            probe_version("g++", args)
                .or_else(|| probe_version("clang++", args))
                .or_else(|| probe_version("cl", args))
        } else if key == "powershell" {
            probe_version("powershell", args)
        } else {
            None
        }
    })?;
    let ver = first_version_token(&line)?;
    // c, c++ and java compilers vary by vendor (gcc/clang, openjdk/temurin),
    // so the display name comes from the output, not the mapping above.
    let display = match name {
        Some(n) => n,
        None => {
            let low = line.to_lowercase();
            match key.as_str() {
                "c" => {
                    if low.contains("clang") {
                        "clang"
                    } else {
                        "cc"
                    }
                }
                "c++" => {
                    if low.contains("clang") {
                        "clang++"
                    } else {
                        "c++"
                    }
                }
                _ => {
                    if low.contains("openjdk") {
                        "openjdk"
                    } else {
                        "java"
                    }
                }
            }
        }
    };
    Some(format!("{} {}", display, ver))
}

// run `<cmd> <args>` and return the first output line, trying stdout then
// stderr (some tools, e.g. java, report on stderr). none on any failure.
fn probe_version(cmd: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(cmd).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = if output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).into_owned()
    } else {
        String::from_utf8_lossy(&output.stdout).into_owned()
    };
    let line = text.lines().next()?.trim();
    if line.is_empty() {
        return None;
    }
    Some(line.to_string())
}

// first whitespace-separated token that looks like a version: leading
// non-digits stripped ("v", "go"), at least one dot ("5.2.21(1)-release"
// yields "5.2.21", "4.3.stable" yields "4.3"). skips a literal "version"
// token ("go version ...").
fn first_version_token(line: &str) -> Option<String> {
    for tok in line.split_whitespace() {
        if tok.eq_ignore_ascii_case("version") {
            continue;
        }
        let stripped: String = tok
            .trim_start_matches(|c: char| !c.is_ascii_digit())
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        let stripped = stripped.trim_matches('.');
        if stripped.contains('.') && stripped.starts_with(|c: char| c.is_ascii_digit()) {
            return Some(stripped.to_string());
        }
    }
    None
}

// toolchain of the login shell itself ($SHELL --version), for shell repos.
fn shell_toolchain() -> Option<String> {
    if cfg!(windows) {
        return windows_shell_toolchain();
    }
    let shell_path = std::env::var("SHELL").ok()?;
    let base = shell_path.rsplit('/').next().filter(|s| !s.is_empty())?;
    let line = probe_version(base, &["--version"])?;
    let ver = first_version_token(&line)?;
    Some(format!("{} {}", base.to_lowercase(), ver))
}

#[cfg(windows)]
fn windows_shell_toolchain() -> Option<String> {
    let shell_path = std::env::var("SHELL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| std::env::var("ComSpec").ok())?;
    let base_raw = shell_path
        .rsplit(['/', '\\'])
        .next()
        .filter(|s| !s.is_empty())?;
    let base = base_raw.trim_end_matches(".exe").trim_end_matches(".EXE");
    if base.is_empty() {
        return None;
    }
    let low = base.to_lowercase();
    if low == "powershell" || low == "powershell_ise" {
        let output = std::process::Command::new(base_raw)
            .args([
                "-NoProfile",
                "-Command",
                "$PSVersionTable.PSVersion.ToString()",
            ])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let line = text.lines().next()?.trim().to_string();
        let ver = first_version_token(&line)?;
        return Some(format!("{} {}", low, ver));
    }
    if low == "cmd" {
        let output = std::process::Command::new(base_raw)
            .args(["/c", "ver"])
            .output()
            .ok()?;
        let text = if output.stdout.is_empty() {
            String::from_utf8_lossy(&output.stderr).into_owned()
        } else {
            String::from_utf8_lossy(&output.stdout).into_owned()
        };
        let line = text.lines().next()?.trim().to_string();
        let ver = first_version_token(&line)?;
        return Some(format!("{} {}", low, ver));
    }
    let line =
        probe_version(base, &["--version"]).or_else(|| probe_version(base_raw, &["--version"]))?;
    let ver = first_version_token(&line)?;
    Some(format!("{} {}", low, ver))
}

#[cfg(not(windows))]
fn windows_shell_toolchain() -> Option<String> {
    None
}

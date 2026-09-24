use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use ignore::WalkBuilder;

use crate::git;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProjectTelemetry {
    pub total_lines: usize,
    pub top_language: Option<(String, f32)>,
    pub coverage_info: String,
}

fn extension_to_lang(ext: &str) -> &'static str {
    match ext {
        "rs" => "rust",
        "py" => "python",
        "go" => "go",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" => "c++",
        "cs" => "c#",
        "gd" | "gdshader" => "gdscript",
        "tscn" | "tres" => "godot",
        "js" | "mjs" | "cjs" => "javascript",
        "ts" | "mts" | "cts" => "typescript",
        "tsx" | "jsx" => "react",
        "vue" => "vue",
        "svelte" => "svelte",
        "html" | "htm" => "html",
        "css" | "scss" | "sass" | "less" => "css",
        "sh" | "bash" | "zsh" | "fish" | "nu" => "shell",
        "ps1" | "psm1" => "powershell",
        "lua" => "lua",
        "zig" => "zig",
        "dart" => "dart",
        "toml" | "yaml" | "yml" | "json" => "config",
        "md" | "markdown" => "markdown",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "swift" => "swift",
        "rb" => "ruby",
        "php" => "php",
        "hs" => "haskell",
        "ex" | "exs" => "elixir",
        "erl" | "hrl" => "erlang",
        "clj" | "cljs" | "cljc" => "clojure",
        "scala" | "sc" => "scala",
        "r" => "r",
        "jl" => "julia",
        "pl" | "pm" => "perl",
        "sql" => "sql",
        "m" | "mm" => "objc",
        "f" | "f90" | "f95" | "for" => "fortran",
        "ml" | "mli" => "ocaml",
        "nim" => "nim",
        "d" => "d",
        "groovy" | "gradle" => "groovy",
        "tf" => "terraform",
        "proto" => "protobuf",
        _ => "other",
    }
}

// files bigger than this are never opened: minified bundles and datasets
// would dominate read time while telling nothing about the project.
const MAX_COUNT_BYTES: u64 = 1024 * 1024;

// walk workspace respecting .gitignore using ignore crate, counting lines of code
pub fn count_lines_of_code(force_count_lines: bool, root: &Path) -> (usize, Option<(String, f32)>) {
    let git_state = git::detect_git_state(root).unwrap_or_default();

    if git_state.is_repo || force_count_lines {
        let mut lang_counts: HashMap<&'static str, usize> = HashMap::new();
        let mut total_lines = 0;

        let walker = WalkBuilder::new(root)
            .standard_filters(true)
            .hidden(true)
            .build();

        for entry in walker.filter_map(Result::ok) {
            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                continue;
            }

            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();

            // unclassified extensions (binaries, media, archives) are skipped
            // without opening: counting their bytes as code would be dishonest,
            // and reading them is what made big directories slow.
            let lang = extension_to_lang(&ext);
            if lang == "other" {
                continue;
            }

            // cheap size gate before paying for a full read
            if entry
                .metadata()
                .map(|m| m.len() > MAX_COUNT_BYTES)
                .unwrap_or(true)
            {
                continue;
            }

            if let Ok(file) = fs::File::open(path) {
                let reader = BufReader::new(file);
                let lines = reader.lines().count();
                if lines > 0 {
                    total_lines += lines;
                    *lang_counts.entry(lang).or_insert(0) += lines;
                }
            }
        }

        if total_lines == 0 {
            return (0, None);
        }

        // top primary code language (ignoring config and markdown if code exists)
        let mut top_code_lang: Option<(&'static str, usize)> = None;

        for (&lang, &count) in &lang_counts {
            if lang != "markdown"
                && lang != "config"
                && lang != "other"
                && top_code_lang.is_none_or(|(_, max)| count > max)
            {
                top_code_lang = Some((lang, count));
            }
        }

        let top_lang = match top_code_lang {
            Some((lang, count)) => {
                let pct = (count as f32 / total_lines as f32) * 100.0;
                Some((lang.to_string(), pct))
            }
            _ => {
                // fall back to overall top language
                let overall = lang_counts.iter().max_by_key(|&(_, &count)| count);
                overall.map(|(&lang, &count)| {
                    let pct = (count as f32 / total_lines as f32) * 100.0;
                    (lang.to_string(), pct)
                })
            }
        };
        (total_lines, top_lang)
    } else {
        (0, None)
    }
}

// detect real coverage report if present in standard locations
pub fn detect_coverage(root: &Path) -> String {
    // 1. check for lcov.info
    let lcov_paths = [
        root.join("lcov.info"),
        root.join("coverage/lcov.info"),
        root.join("target/coverage/lcov.info"),
    ];

    for path in &lcov_paths {
        if !path.is_file() {
            continue;
        }
        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };
        let mut found_lines = 0;
        let mut hit_lines = 0;
        for line in content.lines() {
            if let Some(stripped) = line.strip_prefix("LF:")
                && let Ok(val) = stripped.trim().parse::<usize>()
            {
                found_lines += val;
            } else if let Some(stripped) = line.strip_prefix("LH:")
                && let Ok(val) = stripped.trim().parse::<usize>()
            {
                hit_lines += val;
            }
        }
        if found_lines > 0 {
            let pct = (hit_lines as f32 / found_lines as f32) * 100.0;
            return format!("{:.1}% (lcov.info)", pct);
        }
    }

    // check for tarpaulin-report.json
    let tarpaulin_path = root.join("tarpaulin-report.json");
    if tarpaulin_path.is_file()
        && let Ok(content) = fs::read_to_string(&tarpaulin_path)
        && let Some(idx) = content.find("\"coverage\":")
    {
        let slice = &content[idx + 11..];
        let num_str: String = slice
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        if let Ok(pct) = num_str.parse::<f32>() {
            return format!("{:.1}% (tarpaulin)", pct);
        }
    }

    "not available (no coverage report found)".to_string()
}

// fast top-level language sniff for the tiny card: one capped readdir, no
// recursion, no content reads — the tiny render must never pay for a full
// loc walk. fixed priority order, first marker wins (deterministic).
pub fn detect_project_language(root: &Path) -> Option<String> {
    let names: Vec<String> = fs::read_dir(root)
        .ok()?
        .take(32)
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_lowercase())
        .collect();
    let has = |n: &str| names.iter().any(|f| f == n);
    let has_ext = |ext: &str| names.iter().any(|f| f.ends_with(ext));

    if has("cargo.toml") || has("cargo.lock") {
        return Some("rust".to_string());
    }
    // godot first: a c# godot project has both project.godot and .cs files,
    // and "gdscript" keeps the env row consistent with the loc top language.
    if has("project.godot") || has_ext(".gd") || has_ext(".gdshader") || has_ext(".tscn") {
        return Some("gdscript".to_string());
    }
    if has_ext(".csproj") || has("global.json") || has_ext(".sln") || has_ext(".cs") {
        return Some("c#".to_string());
    }
    if has("pubspec.yaml") || has_ext(".dart") {
        return Some("dart".to_string());
    }
    if has("go.mod") || has_ext(".go") {
        return Some("go".to_string());
    }
    if has("package.json") || has("deno.json") {
        if has("tsconfig.json") || has_ext(".ts") || has_ext(".tsx") {
            return Some("typescript".to_string());
        }
        return Some("javascript".to_string());
    }
    if has("pyproject.toml")
        || has("requirements.txt")
        || has("setup.py")
        || has("pipfile")
        || has_ext(".py")
    {
        return Some("python".to_string());
    }
    if has("pom.xml") || has("build.gradle") || has("build.gradle.kts") || has_ext(".java") {
        return Some("java".to_string());
    }
    if has("package.swift") {
        return Some("swift".to_string());
    }
    if has("gemfile") || has_ext(".rb") {
        return Some("ruby".to_string());
    }
    if has("composer.json") || has_ext(".php") {
        return Some("php".to_string());
    }
    if has("build.zig") || has("build.zig.zon") {
        return Some("zig".to_string());
    }
    if has_ext(".kt") || has_ext(".kts") {
        return Some("kotlin".to_string());
    }
    if has_ext(".cabal") || has("stack.yaml") || has("package.yaml") || has_ext(".hs") {
        return Some("haskell".to_string());
    }
    if has_ext(".lua") {
        return Some("lua".to_string());
    }
    if has_ext(".cpp") || has_ext(".cc") || has_ext(".cxx") || has_ext(".hpp") {
        return Some("c++".to_string());
    }
    if has_ext(".c") || has("cmakelists.txt") || has("makefile") {
        return Some("c".to_string());
    }
    if has("mix.exs") || has_ext(".ex") || has_ext(".exs") {
        return Some("elixir".to_string());
    }
    if has_ext(".scala") || has_ext(".sc") {
        return Some("scala".to_string());
    }
    if has_ext(".clj") || has_ext(".cljs") || has_ext(".cljc") || has("deps.edn") {
        return Some("clojure".to_string());
    }
    if has_ext(".erl") || has_ext(".hrl") || has("rebar.config") {
        return Some("erlang".to_string());
    }
    if has_ext(".r") {
        return Some("r".to_string());
    }
    if has_ext(".jl") {
        return Some("julia".to_string());
    }
    if has_ext(".pl") || has_ext(".pm") || has("cpanfile") {
        return Some("perl".to_string());
    }
    if has_ext(".sql") {
        return Some("sql".to_string());
    }
    if has_ext(".tf") {
        return Some("terraform".to_string());
    }
    if has_ext(".proto") {
        return Some("protobuf".to_string());
    }
    if has_ext(".ps1") || has_ext(".psm1") {
        return Some("powershell".to_string());
    }
    if has_ext(".sh") || has_ext(".fish") || has_ext(".nu") {
        return Some("shell".to_string());
    }
    None
}

pub fn collect_project_telemetry(force_count_lines: bool, root: &Path) -> ProjectTelemetry {
    let (total_lines, top_language) = count_lines_of_code(force_count_lines, root);
    let coverage_info = detect_coverage(root);

    ProjectTelemetry {
        total_lines,
        top_language,
        coverage_info,
    }
}

use std::fs;
use std::path::Path;

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

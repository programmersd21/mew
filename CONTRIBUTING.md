# contributing to mew

mew is a fast terminal card for your project and its git state. no daemons, no background anything, one render per invocation.

## development loop

all pull requests must pass formatting, the linter, and the test suite cleanly (same as ci):

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

## conventions

- all code comments, documentation, and user-facing messages stay lowercase. the readme is lowercase too; keep it that way.
- no unicode emoji anywhere: cli text, markdown, status output. nerd font glyphs are fine; they are not emoji.
- use conventional commit prefixes (`feat:`, `fix:`, `docs:`, `perf:`, `refactor:`, `test:`, `chore:`).
- no fake telemetry: every number displayed comes from a real source, or the card says so. never invent a plausible value.
- `mew theme` is the single source of truth for the default `theme.toml` template. never paste a hardcoded copy of it into scripts or docs; pipe the subcommand instead.
- plain mode must stay byte-clean: piped output and `NO_COLOR` sessions get zero escape bytes, same text otherwise.
- help, version, and error screens (`src/clap_ui.rs`) wear the same palette as the card. new surfaces wire into it instead of ad-hoc styling.

## what mew is not

- not a system monitor. os, uptime, toolchain: yes, at a glance. cpu/ram/disk history graphs: no.
- not a daemon. it renders once and exits.
- not a fastfetch clone. fastfetch shows off your machine; mew shows you your afternoon.

## bug reports

open an issue with your terminal emulator, font, os, and what you saw vs what you expected. a screenshot beats a paragraph every time.

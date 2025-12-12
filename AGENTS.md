# Repository Guidelines

## Project Structure & Module Organization
- `src/` — Application code. Key areas: `app/` (orchestration), `ui_components/` (UI widgets), `screens/` (views), `api/` (SoundCloud API), `utils/` (audio, OAuth, caching, shaders), `models/`, `shaders/`, `assets/`.
- `testunits/` — Standalone executable tests and experiments (see docs/TESTUNIT.md, testunits/README.md).
- `docs/` — Architecture notes, pipeline specs, setup guides.
- `images/` — Screenshots used in docs/README.
- Build configuration in `Cargo.toml`, build-time env in `build.rs`.

## Build, Test, and Development Commands
- Configure env once: `cp .env.example .env` and fill `SOUNDCLOUD_CLIENT_ID`/`SOUNDCLOUD_CLIENT_SECRET`.
- Run debug: `cargo run` — builds and launches `TempRS`.
- Release build: `cargo build --release` then run `./target/release/TempRS`.
- Lint/format: `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings`.
- AppImage (Linux): `./create_appimage.sh`.
- Windows helper: `./build_windows.sh`.

## Coding Style & Naming Conventions
- Rust 2021 edition; 4-space indentation; no tabs.
- Use `rustfmt` defaults; run `cargo fmt` before pushing.
- Prefer `snake_case` for files/modules/functions, `PascalCase` for types/structs/enums, `SCREAMING_SNAKE_CASE` for consts.
- Keep modules focused; colocate helpers with their domain (e.g., UI utils in `ui_components/helpers.rs`).

## Testing Guidelines
- Framework: executable test bins under `testunits/` (see `docs/TESTUNIT.md`).
- Run a test bin (example):
  - Add to Cargo (example):
    [[bin]] name = "shader_test" path = "testunits/shader_test.rs"
  - Execute: `cargo run --release --bin shader_test`.
- Name test bins descriptively (e.g., `play_history_test.rs`). Keep output informative.

## Commit & Pull Request Guidelines
- Commits: present-tense, imperative, scoped. Examples: `feat(audio): add progressive decoder`, `fix(ui): prevent duplicate widget ids`.
- Group related changes; keep diffs minimal and focused.
- PRs must include: clear description, rationale, before/after notes or screenshots for UI changes, and linked issues.
- Ensure `cargo fmt` and `cargo clippy` pass; include quick manual run notes if touching audio/network/shader code.

## Security & Configuration
- Do not commit secrets. `.env` is loaded at build time via `build.rs`.
- Cache/config paths: `~/.config/TempRS/` and `~/.cache/TempRS/` are created at runtime; avoid hardcoding alternate locations.

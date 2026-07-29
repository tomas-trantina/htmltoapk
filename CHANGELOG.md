# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-07-29

### Added

- `htmltoapk make <input.html> <output.apk>` — build an Android APK from a single-file HTML document.
- `htmltoapk make-dir <dir> <output.apk>` — build from an HTML/CSS/JS web directory with automatic entry-point detection.
- `htmltoapk setup` — create the default configuration and the workspace/log directories.
- `htmltoapk config` / `config get` / `config set` / `config path` / `config reset` / `config dump` — 29 typed configuration keys with validation.
- `htmltoapk doctor` — environment check (node, npm, npx, java, Android SDK, platform tools) with `--json` output.
- `htmltoapk clean` — remove generated workspaces and build logs, with `--dry-run`.
- `htmltoapk zip` — archive any directory, skipping `node_modules`, `.gradle`, `target` and friends.
- TUI (launched when `htmltoapk` runs without arguments) with build wizard, live stage progress and streamed build log, configuration editor, doctor screen, ZIP export, cleanup screen and built-in help.
- Seven-stage build pipeline shared by the CLI and the TUI: preflight, workspace, dependencies, android platform, android configuration, gradle build, package.
- Capacitor workspace generation (`package.json`, `capacitor.config.json`, `www/`, Android platform), SDK level patching, app name/version patching, optional keystore-based release signing.
- Icon and splash generation through `@capacitor/assets`, with graceful fallback to Capacitor defaults.
- Smart naming: app name derived from the HTML `<title>` or the path, application id derived from `appIdPrefix`.
- Structured error layer with actionable hints and dedicated exit codes for missing tools, missing Android SDK, bad input, invalid configuration and failed builds.
- `install.sh`, `uninstall.sh`, example configuration, example web app and MIT license.

[0.1.0]: https://github.com/tarzohoss/htmltoapk/releases/tag/v0.1.0

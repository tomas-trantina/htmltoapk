<h1 align="center">htmltoapk</h1>

<p align="center">
	<b>Turn any HTML website into an Android APK.</b><br>
	A single Rust binary that wraps Capacitor with a friendly CLI and a modern terminal UI.
</p>

<img width="1167" height="681" alt="image" src="https://github.com/user-attachments/assets/c5493f1f-e25f-4389-a00a-3bc77c8b0ebb" />
<img width="575" height="646" alt="image" src="https://github.com/user-attachments/assets/f5267805-f9b5-4436-bd11-db628fc9b383" />

<p align="center">
	<img alt="Rust" src="https://img.shields.io/badge/rust-stable-000?logo=rust">
	<img alt="Platform" src="https://img.shields.io/badge/platform-Linux-1f6feb">
	<img alt="License" src="https://img.shields.io/badge/license-MIT-green">
</p>

---

## What it does

`htmltoapk` takes a single-file `index.html` or a whole `HTML/CSS/JS` directory and produces a
ready-to-install `.apk`. It scaffolds a valid Capacitor workspace, installs the Android platform,
applies your icon and splash, runs Gradle, and copies the resulting APK where you asked for it.

Run it with arguments for scripting, or run it with **no arguments** to open the TUI.

## Features

- **One binary, two front-ends** - a scriptable CLI and a full-featured terminal UI sharing the same core.
- **Single file or whole directory** - `make app.html out.apk` and `make-dir ./site out.apk`.
- **Smart naming** - the app name comes from `<title>` or the file name, the application id from `appIdPrefix` + a sanitised slug (`com.user.recipebox`), with Java keywords and digits handled.
- **Global configuration** - 29 documented keys (prefix, default name, build type, workspace, output dir, signing, icon/splash, SDK levels, Capacitor and npm options).
- **Interactive gap filling** - `make` asks only for what is missing and offers config-based defaults; `--yes` keeps it fully non-interactive for CI.
- **Environment doctor** - checks `node`, `npm`, `npx`, `java`, `ANDROID_HOME`, platform tools and licences, with copy-pasteable fixes and `--json` output.
- **Debug or release** - debug signing out of the box, keystore signing with passwords read from environment variables.
- **Icon and splash generation** - optional `@capacitor/assets` pass with a configurable background colour.
- **Workspace hygiene** - reusable workspaces for fast rebuilds, `clean` to reclaim space, `zip` to archive any project.
- **Readable failures** - every error has a headline, the failing stage, the exit code, the log path, the last log lines and a hint.

## Requirements

| Tool | Version | Why |
| --- | --- | --- |
| Rust | stable (1.76+) | building `htmltoapk` |
| Node.js | 18+ | Capacitor CLI |
| npm + npx | 9+ | dependency install, `@capacitor/assets` |
| JDK | 17 | Gradle / Android build |
| Android SDK | platform 34 + build-tools | `ANDROID_HOME` or `ANDROID_SDK_ROOT` |

Check everything at once:

```bash
htmltoapk doctor
```

## Install

```bash
git clone https://github.com/your-name/htmltoapk.git
cd htmltoapk
./install.sh
```

The installer builds a release binary, installs it to `~/.local/bin/htmltoapk`, warns if that
directory is not on your `PATH`, and offers to run the first-time setup.

Options: `--prefix <dir>`, `--bin-dir <dir>`, `--debug`, `--no-setup`.

Manual build:

```bash
cargo build --release
install -m 0755 target/release/htmltoapk ~/.local/bin/htmltoapk
htmltoapk setup
```

Uninstall:

```bash
./uninstall.sh          # remove the binary
./uninstall.sh --purge  # also remove config, workspaces and logs
```

## Usage

```text
htmltoapk                              open the terminal UI
htmltoapk setup                        create the configuration file
htmltoapk config                       show the effective configuration
htmltoapk config get <key>             print one value
htmltoapk config set <key> <value>     change one value
htmltoapk config path|dump|reset       file path, raw TOML, restore defaults
htmltoapk make <input> [output.apk]    build from a file or directory
htmltoapk make-dir <dir> [output.apk]  build from a web directory
htmltoapk doctor [--json]              verify the toolchain
htmltoapk clean [--dry-run]            remove workspaces and logs
htmltoapk zip [path] [-o out.zip]      archive a project
```

Useful `make` flags:

| Flag | Meaning |
| --- | --- |
| `--name <name>` | application name |
| `--id <app.id>` | full application id (overrides `appIdPrefix`) |
| `--build-type <debug\|release>` | Gradle variant |
| `--icon <file>` / `--splash <file>` | source images |
| `--zip` | also archive the generated workspace |
| `--keep-workspace` / `--discard-workspace` | override `keepWorkspace` |
| `--offline` | pass `--offline` to npm |
| `--yes` | never prompt (CI mode) |

### Terminal UI

Start it with `htmltoapk`. Everything is reachable without arguments:

- **Build APK** - form for input, name, id, output, variant, icon, splash, with a live 7-stage progress gauge and streaming log.
- **Configuration** - browse and edit every key, `s` saves, `r` reloads.
- **Doctor** - re-runnable environment report.
- **Export ZIP** - archive a directory, skipping `node_modules`, `.git` and build output.
- **Clean** - list reclaimable workspaces and logs, delete with `d`.
- **Help** - keys, CLI equivalents and requirements.

Keys: `Up`/`Down`/`Tab` move, `Enter` confirms, `Esc` goes back, `Ctrl+U` clears a field, `q` quits.

## Configuration

The config lives at `~/.config/htmltoapk/config.toml` (see `examples/config.toml`).
Workspaces and logs live under `~/.local/share/htmltoapk/`.

| Key | Default | Description |
| --- | --- | --- |
| `appIdPrefix` | `com.example` | package prefix, needs at least two lowercase segments |
| `appName` | `My App` | fallback application name |
| `buildType` | `debug` | default Gradle variant |
| `workspace` | `~/.local/share/htmltoapk/workspaces` | where Capacitor projects are generated |
| `outputDir` | `.` | default APK destination |
| `keepWorkspace` | `true` | reuse workspaces for faster rebuilds |
| `autoNaming` | `true` | derive names from `<title>` / file names |
| `signing` | `debug` | `none`, `debug` or `keystore` |
| `keystore.path` / `.alias` | unset | release keystore |
| `keystore.storePasswordEnv` / `.keyPasswordEnv` | `HTMLTOAPK_STORE_PASSWORD` / `HTMLTOAPK_KEY_PASSWORD` | env vars holding the passwords |
| `assets.icon` / `assets.splash` | unset | default source images |
| `assets.backgroundColor` | `#0B0F17` | splash background |
| `assets.generate` | `true` | run `@capacitor/assets` |
| `android.minSdk` / `targetSdk` / `compileSdk` | `23` / `34` / `34` | SDK levels |
| `android.versionName` / `versionCode` | `1.0.0` / `1` | app version |
| `capacitor.version` | `6` | Capacitor major version |
| `capacitor.webDir` | `www` | web assets directory inside the workspace |
| `capacitor.androidScheme` | `https` | WebView scheme |
| `capacitor.allowMixedContent` | `true` | allow mixed content in the WebView |
| `capacitor.plugins` | `[]` | extra npm packages per workspace |
| `npm.offline` / `npm.registry` / `npm.extraArgs` | `false` / unset / `[]` | npm behaviour |

```bash
htmltoapk config set appIdPrefix com.user
htmltoapk config set signing keystore
htmltoapk config set keystore.path ~/keys/release.jks
htmltoapk config get android.targetSdk
```

## Examples

Single HTML file, everything derived from the config:

```bash
htmltoapk make ./notes.html
# -> ./notes-debug.apk, app id com.user.notes
```

Web directory with explicit metadata:

```bash
htmltoapk make-dir ./examples/hello ~/apks/hello.apk \
	--name "Hello Capacitor" --id com.user.hello --icon ~/brand/icon.png
```

Signed release build in CI:

```bash
export HTMLTOAPK_STORE_PASSWORD=...
export HTMLTOAPK_KEY_PASSWORD=...
htmltoapk config set signing keystore
htmltoapk make-dir ./dist ./out/app-release.apk --build-type release --yes
```

Housekeeping:

```bash
htmltoapk doctor --json | jq '.checks[] | select(.status != "ok")'
htmltoapk clean --dry-run
htmltoapk zip ./my-project -o my-project.zip
```

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| `node is required but was not found` | install Node.js 18+ (`nvm install 20` or your package manager) |
| `npm` / `npx` missing | reinstall Node.js; `npx` ships with npm 9+ |
| `java is required but was not found` | install JDK 17 (`sudo apt install openjdk-17-jdk`) and set `JAVA_HOME` |
| `Android SDK was not found` | install the SDK, then `export ANDROID_HOME=$HOME/Android/Sdk` and add `platform-tools` to `PATH` |
| `Gradle build failed` | the error shows the stage, exit code, log path and last lines - open the log for the full Gradle output |
| `input does not exist` | pass a path to an `.html` file or a directory containing one |
| `no HTML entry point` | make sure the directory has `index.html` (or another `.html` file) |
| `configuration is invalid` | run `htmltoapk config` to see the offending key, or `htmltoapk config reset` |
| release build unsigned | set `signing = "keystore"` plus `keystore.*` and export the password env vars |
| slow first build | the initial `npm install` and Gradle download are cached; keep `keepWorkspace = true` |
| disk filling up | `htmltoapk clean` removes old workspaces and logs |

Exit codes: `0` ok, `3` missing tool, `4` Android SDK, `5` bad input, `6` invalid config,
`7` build failed, `8` I/O error, `130` cancelled.

## Development

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -- doctor
cargo run            # TUI
```

Layout:

```text
src/
	main.rs          entry point, TUI when no arguments are given
	cli.rs           clap command definitions and usage texts
	commands.rs      CLI dispatch, prompts, summaries
	error.rs         error type with headlines, hints and exit codes
	core/            UI-free logic
		config.rs      TOML config, keys, validation, resolution
		input.rs       single-file vs directory input, entry detection
		naming.rs      slugs, app names, application ids
		workspace.rs   Capacitor scaffolding, Gradle patching, signing
		build.rs       the 7-stage build pipeline
		assets.rs      icon and splash handling
		doctor.rs      environment checks
		clean.rs       workspace and log cleanup
		zipper.rs      ZIP export
		process.rs     command execution and log streaming
		fsx.rs         filesystem helpers
		paths.rs       XDG paths
		report.rs      progress reporter trait
	ui/
		cli.rs         colours, prompts, reporter, error rendering
		tui/           ratatui app, state, theme, rendering
```

Both front-ends call the same `core::build::run` through the `Reporter` trait, so adding a
command means adding a `core` function plus a thin CLI/TUI binding. Adding a config key means
extending `Config`, `KEYS` and `describe` - the CLI and the TUI editor pick it up automatically.

## License

MIT - see [LICENSE](LICENSE).

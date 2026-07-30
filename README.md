<div align="center">

<h1>
  <img src="https://github.com/user-attachments/assets/c5493f1f-e25f-4389-a00a-3bc77c8b0ebb" alt="htmltoapk banner" width="100%">
</h1>

<h3>📦 Turn any HTML website into an Android APK</h3>

<p>A single Rust binary that wraps Capacitor with a friendly CLI <em>and</em> a full-featured terminal UI.</p>

<p>
  <img alt="Rust" src="https://img.shields.io/badge/rust-1.76%2B-orange?logo=rust&logoColor=white">
  <img alt="Platform" src="https://img.shields.io/badge/platform-Linux-1f6feb?logo=linux&logoColor=white">
  <img alt="License" src="https://img.shields.io/badge/license-MIT-22c55e">
  <img alt="Version" src="https://img.shields.io/badge/version-0.1.0-8b5cf6">
  <img alt="Capacitor" src="https://img.shields.io/badge/capacitor-v6-119eff?logo=capacitor&logoColor=white">
</p>

</div>

---

## ✨ What it does

`htmltoapk` takes a single-file `index.html` **or** a whole `HTML/CSS/JS` directory and produces a
ready-to-install `.apk`. It handles the entire pipeline for you:

1. 🏗️ Scaffolds a valid Capacitor workspace
2. 📦 Installs the Android platform
3. 🎨 Applies your icon and splash screen
4. ⚙️ Runs Gradle
5. 📤 Copies the resulting APK to your destination

Run it with **arguments** for scripting/CI, or run it with **no arguments** to open the interactive TUI.

---

## 🖼️ Screenshots

<div align="center">
<img width="575" height="646" alt="Terminal UI" src="https://github.com/user-attachments/assets/f5267805-f9b5-4436-bd11-db628fc9b383">
</div>

---

## 🚀 Features

| Feature | Description |
|---|---|
| 🖥️ **One binary, two front-ends** | Scriptable CLI and a full-featured TUI sharing the same core |
| 📄 **Single file or whole directory** | `make app.html out.apk` and `make-dir ./site out.apk` |
| 🏷️ **Smart naming** | App name from `<title>` or filename, ID from `appIdPrefix` + sanitised slug |
| ⚙️ **Global configuration** | 29 documented keys — prefix, signing, icons, SDK levels, Capacitor options and more |
| 🤖 **Interactive gap filling** | Asks only for what is missing; `--yes` for fully non-interactive CI mode |
| 🩺 **Environment doctor** | Checks `node`, `npm`, `npx`, `java`, `ANDROID_HOME`, platform tools and licences |
| 🔐 **Debug or release** | Debug signing out of the box; keystore signing via environment variables |
| 🎨 **Icon & splash generation** | Optional `@capacitor/assets` pass with configurable background colour |
| 🧹 **Workspace hygiene** | Reusable workspaces for fast rebuilds, `clean` to reclaim space, `zip` to archive |
| ❌ **Readable failures** | Every error has a headline, stage, exit code, log path, last log lines and a fix hint |

---

## 📋 Requirements

| Tool | Version | Purpose |
|---|---|---|
| **Rust** | stable (1.76+) | Building `htmltoapk` |
| **Node.js** | 18+ | Capacitor CLI |
| **npm + npx** | 9+ | Dependency install & `@capacitor/assets` |
| **JDK** | 17 | Gradle / Android build |
| **Android SDK** | platform 34 + build-tools | `ANDROID_HOME` or `ANDROID_SDK_ROOT` |

Verify everything at once:

```bash
htmltoapk doctor
```

---

## 📥 Install

### Quick install (recommended)

```bash
git clone https://github.com/tarzohoss/htmltoapk.git
cd htmltoapk
./install.sh
```

The installer builds a release binary, installs it to `~/.local/bin/htmltoapk`, warns if that
directory is not on your `PATH`, and offers to run first-time setup.

**Options:** `--prefix <dir>`, `--bin-dir <dir>`, `--debug`, `--no-setup`

### Manual build

```bash
cargo build --release
install -m 0755 target/release/htmltoapk ~/.local/bin/htmltoapk
htmltoapk setup
```

### Uninstall

```bash
./uninstall.sh          # remove the binary
./uninstall.sh --purge  # also remove config, workspaces and logs
```

---

## 🛠️ Usage

```
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

### `make` / `make-dir` flags

| Flag | Meaning |
|---|---|
| `--name <name>` | application name |
| `--id <app.id>` | full application id (overrides `appIdPrefix`) |
| `--build-type <debug\|release>` | Gradle variant |
| `--icon <file>` / `--splash <file>` | source images |
| `--zip` | also archive the generated workspace |
| `--keep-workspace` / `--discard-workspace` | override `keepWorkspace` |
| `--offline` | pass `--offline` to npm |
| `--yes` | never prompt (CI mode) |

### Terminal UI

Launch with `htmltoapk` (no arguments). Navigate with `↑`/`↓`/`Tab`, confirm with `Enter`, go back with `Esc`, quit with `q`.

| Screen | Description |
|---|---|
| **Build APK** | Form for input, name, id, output, variant, icon, splash — live 7-stage progress & streaming log |
| **Configuration** | Browse and edit every key; `s` saves, `r` reloads |
| **Doctor** | Re-runnable environment report |
| **Export ZIP** | Archive a directory, skipping `node_modules`, `.git` and build output |
| **Clean** | List reclaimable workspaces and logs, delete with `d` |
| **Help** | Keys, CLI equivalents and requirements |

---

## ⚙️ Configuration

Config file: `~/.config/htmltoapk/config.toml` (see [`examples/config.toml`](examples/))  
Workspaces & logs: `~/.local/share/htmltoapk/`

<details>
<summary><strong>📋 All configuration keys</strong></summary>
<br>

| Key | Default | Description |
|---|---|---|
| `appIdPrefix` | `com.example` | Package prefix — needs at least two lowercase segments |
| `appName` | `My App` | Fallback application name |
| `buildType` | `debug` | Default Gradle variant |
| `workspace` | `~/.local/share/htmltoapk/workspaces` | Where Capacitor projects are generated |
| `outputDir` | `.` | Default APK destination |
| `keepWorkspace` | `true` | Reuse workspaces for faster rebuilds |
| `autoNaming` | `true` | Derive names from `<title>` / file names |
| `signing` | `debug` | `none`, `debug` or `keystore` |
| `keystore.path` / `.alias` | unset | Release keystore |
| `keystore.storePasswordEnv` | `HTMLTOAPK_STORE_PASSWORD` | Env var holding the store password |
| `keystore.keyPasswordEnv` | `HTMLTOAPK_KEY_PASSWORD` | Env var holding the key password |
| `assets.icon` / `assets.splash` | unset | Default source images |
| `assets.backgroundColor` | `#0B0F17` | Splash background colour |
| `assets.generate` | `true` | Run `@capacitor/assets` |
| `android.minSdk` | `23` | Minimum SDK level |
| `android.targetSdk` | `34` | Target SDK level |
| `android.compileSdk` | `34` | Compile SDK level |
| `android.versionName` | `1.0.0` | App version string |
| `android.versionCode` | `1` | App version integer |
| `capacitor.version` | `6` | Capacitor major version |
| `capacitor.webDir` | `www` | Web assets directory inside the workspace |
| `capacitor.androidScheme` | `https` | WebView scheme |
| `capacitor.allowMixedContent` | `true` | Allow mixed content in the WebView |
| `capacitor.plugins` | `[]` | Extra npm packages per workspace |
| `npm.offline` | `false` | Pass `--offline` to npm |
| `npm.registry` | unset | Custom npm registry |
| `npm.extraArgs` | `[]` | Extra args for npm |

</details>

```bash
# Common config tweaks
htmltoapk config set appIdPrefix com.yourname
htmltoapk config set signing keystore
htmltoapk config set keystore.path ~/keys/release.jks
htmltoapk config get android.targetSdk
```

---

## 💡 Examples

**Single HTML file — everything derived from config:**

```bash
htmltoapk make ./notes.html
# → ./notes-debug.apk, app id com.user.notes
```

**Web directory with explicit metadata:**

```bash
htmltoapk make-dir ./examples/hello ~/apks/hello.apk \
  --name "Hello Capacitor" --id com.user.hello --icon ~/brand/icon.png
```

**Signed release build in CI:**

```bash
export HTMLTOAPK_STORE_PASSWORD=secret
export HTMLTOAPK_KEY_PASSWORD=secret
htmltoapk config set signing keystore
htmltoapk make-dir ./dist ./out/app-release.apk --build-type release --yes
```

**Housekeeping:**

```bash
htmltoapk doctor --json | jq '.checks[] | select(.status != "ok")'
htmltoapk clean --dry-run
htmltoapk zip ./my-project -o my-project.zip
```

---

## 🔧 Troubleshooting

| Symptom | Fix |
|---|---|
| `node is required but was not found` | Install Node.js 18+ (`nvm install 20` or your package manager) |
| `npm` / `npx` missing | Reinstall Node.js — `npx` ships with npm 9+ |
| `java is required but was not found` | Install JDK 17 (`sudo apt install openjdk-17-jdk`) and set `JAVA_HOME` |
| `Android SDK was not found` | Install the SDK, then `export ANDROID_HOME=$HOME/Android/Sdk` and add `platform-tools` to `PATH` |
| `Gradle build failed` | The error shows the stage, exit code, log path and last lines — open the log for the full Gradle output |
| `input does not exist` | Pass a path to an `.html` file or a directory containing one |
| `no HTML entry point` | Make sure the directory has `index.html` (or another `.html` file) |
| `configuration is invalid` | Run `htmltoapk config` to see the offending key, or `htmltoapk config reset` |
| Release build unsigned | Set `signing = "keystore"` plus `keystore.*` and export the password env vars |
| Slow first build | The initial `npm install` and Gradle download are cached — keep `keepWorkspace = true` |
| Disk filling up | `htmltoapk clean` removes old workspaces and logs |

**Exit codes:** `0` ok · `3` missing tool · `4` Android SDK · `5` bad input · `6` invalid config · `7` build failed · `8` I/O error · `130` cancelled

---

## 🏗️ Development

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -- doctor
cargo run            # TUI
```

### Project layout

```
src/
├── main.rs          entry point, TUI when no arguments are given
├── cli.rs           clap command definitions and usage texts
├── commands.rs      CLI dispatch, prompts, summaries
├── error.rs         error type with headlines, hints and exit codes
├── core/            UI-free logic
│   ├── config.rs    TOML config, keys, validation, resolution
│   ├── input.rs     single-file vs directory input, entry detection
│   ├── naming.rs    slugs, app names, application ids
│   ├── workspace.rs Capacitor scaffolding, Gradle patching, signing
│   ├── build.rs     the 7-stage build pipeline
│   ├── assets.rs    icon and splash handling
│   ├── doctor.rs    environment checks
│   ├── clean.rs     workspace and log cleanup
│   ├── zipper.rs    ZIP export
│   ├── process.rs   command execution and log streaming
│   ├── fsx.rs       filesystem helpers
│   ├── paths.rs     XDG paths
│   └── report.rs    progress reporter trait
└── ui/
    ├── cli.rs       colours, prompts, reporter, error rendering
    └── tui/         ratatui app, state, theme, rendering
```

> Both front-ends call the same `core::build::run` through the `Reporter` trait, so adding a
> command means adding a `core` function plus a thin CLI/TUI binding. Adding a config key means
> extending `Config`, `KEYS` and `describe` — the CLI and TUI editor pick it up automatically.

---

## 📄 License

MIT — see [LICENSE](LICENSE).

---

<div align="center">
<sub>Made with ❤️ and Rust</sub>
</div>

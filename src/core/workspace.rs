//! Capacitor workspace generation and Gradle project patching.

use std::path::{Path, PathBuf};

use serde_json::json;

use crate::core::build::BuildRequest;
use crate::core::config::Signing;
use crate::core::fsx;
use crate::error::{Error, Result};

/// Layout of a generated workspace.
#[derive(Debug, Clone)]
pub struct Workspace {
    pub root: PathBuf,
    pub web_dir: PathBuf,
    pub android: PathBuf,
}

impl Workspace {
    /// Has `npx cap add android` already run here?
    pub fn has_android(&self) -> bool {
        self.android.join("app").is_dir() && self.android.join("gradlew").is_file()
    }

    /// Path of the Gradle wrapper.
    pub fn gradlew(&self) -> PathBuf {
        self.android.join("gradlew")
    }

    /// Directory holding the built APKs for a variant.
    pub fn apk_dir(&self, build_type: &str) -> PathBuf {
        self.android
            .join("app")
            .join("build")
            .join("outputs")
            .join("apk")
            .join(build_type)
    }
}

/// Create (or refresh) the Capacitor workspace for a build request.
pub fn scaffold(request: &BuildRequest) -> Result<Workspace> {
    let root = request.workspace_dir();
    fsx::create_dir_all(&root)?;

    let web_dir = root.join(&request.capacitor_web_dir);
    let workspace = Workspace {
        root: root.clone(),
        web_dir: web_dir.clone(),
        android: root.join("android"),
    };

    write_package_json(request, &root)?;
    write_capacitor_config(request, &root)?;
    write_workspace_metadata(request, &root)?;
    write_gitignore(&root)?;

    // Web assets are always refreshed so rebuilds pick up source changes.
    fsx::remove_dir_all(&web_dir)?;
    request.input.materialize(&web_dir)?;

    let index = web_dir.join("index.html");
    if !index.is_file() {
        return Err(Error::InvalidInput {
            path: request.input.display_path().to_path_buf(),
            reason: "no index.html could be produced for the Capacitor web directory".to_string(),
        });
    }

    Ok(workspace)
}

fn write_package_json(request: &BuildRequest, root: &Path) -> Result<()> {
    let range = format!("^{}", request.capacitor_version.trim());
    let mut dependencies = serde_json::Map::new();
    dependencies.insert("@capacitor/core".to_string(), json!(range));
    dependencies.insert("@capacitor/android".to_string(), json!(range));
    for plugin in &request.capacitor_plugins {
        let plugin = plugin.trim();
        if !plugin.is_empty() {
            dependencies.insert(plugin.to_string(), json!("latest"));
        }
    }

    let package = json!({
        "name": request.slug(),
        "version": request.version_name,
        "private": true,
        "description": format!("{} packaged by htmltoapk", request.app_name),
        "scripts": {
            "sync": "cap sync android",
            "build:debug": "cd android && ./gradlew assembleDebug",
            "build:release": "cd android && ./gradlew assembleRelease"
        },
        "dependencies": dependencies,
        "devDependencies": {
            "@capacitor/cli": range
        }
    });

    let text = serde_json::to_string_pretty(&package)
        .map_err(|err| Error::other(format!("could not render package.json: {err}")))?;
    fsx::write(&root.join("package.json"), &format!("{text}\n"))
}

fn write_capacitor_config(request: &BuildRequest, root: &Path) -> Result<()> {
    let config = json!({
        "appId": request.app_id,
        "appName": request.app_name,
        "webDir": request.capacitor_web_dir,
        "bundledWebRuntime": false,
        "android": {
            "allowMixedContent": request.allow_mixed_content
        },
        "server": {
            "androidScheme": request.android_scheme
        }
    });
    let text = serde_json::to_string_pretty(&config)
        .map_err(|err| Error::other(format!("could not render capacitor.config.json: {err}")))?;
    fsx::write(&root.join("capacitor.config.json"), &format!("{text}\n"))
}

fn write_workspace_metadata(request: &BuildRequest, root: &Path) -> Result<()> {
    let meta = json!({
        "generator": "htmltoapk",
        "generatorVersion": env!("CARGO_PKG_VERSION"),
        "appName": request.app_name,
        "appId": request.app_id,
        "buildType": request.build_type.as_str(),
        "source": request.input.display_path().display().to_string(),
        "createdAt": crate::core::paths::unix_secs(),
    });
    let text = serde_json::to_string_pretty(&meta)
        .map_err(|err| Error::other(format!("could not render workspace metadata: {err}")))?;
    fsx::write(&root.join("htmltoapk.json"), &format!("{text}\n"))
}

fn write_gitignore(root: &Path) -> Result<()> {
    let body = "node_modules/\nandroid/.gradle/\nandroid/build/\nandroid/app/build/\nandroid/local.properties\n*.apk\n*.aab\n";
    fsx::write(&root.join(".gitignore"), body)
}

/// Point the Gradle project at the SDK found by the doctor.
pub fn write_local_properties(workspace: &Workspace, sdk_root: &Path) -> Result<()> {
    let body = format!("sdk.dir={}\n", sdk_root.display());
    fsx::write(&workspace.android.join("local.properties"), &body)
}

/// Patch `android/variables.gradle` with the configured SDK levels.
pub fn apply_sdk_levels(
    workspace: &Workspace,
    min_sdk: u32,
    target_sdk: u32,
    compile_sdk: u32,
) -> Result<()> {
    let path = workspace.android.join("variables.gradle");
    if !path.is_file() {
        return Ok(());
    }
    let mut text = fsx::read_to_string(&path)?;
    text = replace_gradle_number(&text, "minSdkVersion", min_sdk);
    text = replace_gradle_number(&text, "targetSdkVersion", target_sdk);
    text = replace_gradle_number(&text, "compileSdkVersion", compile_sdk);
    fsx::write(&path, &text)
}

fn replace_gradle_number(text: &str, key: &str, value: u32) -> String {
    text.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with(key) && trimmed[key.len()..].trim_start().starts_with('=') {
                let indent: String = line
                    .chars()
                    .take_while(|ch| ch.is_whitespace())
                    .collect();
                format!("{indent}{key} = {value}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

/// Patch the visible application name and version metadata.
pub fn apply_app_metadata(
    workspace: &Workspace,
    app_name: &str,
    version_name: &str,
    version_code: u32,
) -> Result<()> {
    let strings = workspace
        .android
        .join("app/src/main/res/values/strings.xml");
    if strings.is_file() {
        let text = fsx::read_to_string(&strings)?;
        let escaped = escape_xml(app_name);
        let text = replace_xml_string(&text, "app_name", &escaped);
        let text = replace_xml_string(&text, "title_activity_main", &escaped);
        fsx::write(&strings, &text)?;
    }

    let gradle = workspace.android.join("app/build.gradle");
    if gradle.is_file() {
        let text = fsx::read_to_string(&gradle)?;
        let text = replace_gradle_assignment(&text, "versionName", &format!("\"{version_name}\""));
        let text = replace_gradle_assignment(&text, "versionCode", &version_code.to_string());
        fsx::write(&gradle, &text)?;
    }
    Ok(())
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn replace_xml_string(text: &str, name: &str, value: &str) -> String {
    let opening = format!("<string name=\"{name}\">");
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(&opening) {
        let after = start + opening.len();
        let end = match rest[after..].find("</string>") {
            Some(offset) => after + offset,
            None => break,
        };
        out.push_str(&rest[..after]);
        out.push_str(value);
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

fn replace_gradle_assignment(text: &str, key: &str, value: &str) -> String {
    text.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with(key) {
                let indent: String = line
                    .chars()
                    .take_while(|ch| ch.is_whitespace())
                    .collect();
                format!("{indent}{key} {value}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

const SIGNING_MARKER: &str = "// htmltoapk:signing";

/// Inject a release `signingConfig` backed by a user keystore.
pub fn apply_release_signing(
    workspace: &Workspace,
    keystore: &Path,
    alias: &str,
    store_password: &str,
    key_password: &str,
) -> Result<()> {
    let gradle = workspace.android.join("app/build.gradle");
    if !gradle.is_file() {
        return Err(Error::other(
            "android/app/build.gradle is missing, cannot configure signing",
        ));
    }
    let properties = format!(
        "storeFile={}\nstorePassword={}\nkeyAlias={}\nkeyPassword={}\n",
        keystore.display(),
        store_password,
        alias,
        key_password
    );
    fsx::write(
        &workspace.android.join("htmltoapk-signing.properties"),
        &properties,
    )?;

    let text = fsx::read_to_string(&gradle)?;
    if text.contains(SIGNING_MARKER) {
        return Ok(());
    }

    let block = format!(
        "{SIGNING_MARKER}\n\
         android {{\n\
         \x20   signingConfigs {{\n\
         \x20       htmltoapk {{\n\
         \x20           def props = new Properties()\n\
         \x20           def propsFile = rootProject.file('htmltoapk-signing.properties')\n\
         \x20           if (propsFile.exists()) {{\n\
         \x20               propsFile.withInputStream {{ stream -> props.load(stream) }}\n\
         \x20               storeFile file(props['storeFile'])\n\
         \x20               storePassword props['storePassword']\n\
         \x20               keyAlias props['keyAlias']\n\
         \x20               keyPassword props['keyPassword']\n\
         \x20           }}\n\
         \x20       }}\n\
         \x20   }}\n\
         \x20   buildTypes {{\n\
         \x20       release {{\n\
         \x20           signingConfig signingConfigs.htmltoapk\n\
         \x20       }}\n\
         \x20   }}\n\
         }}\n"
    );
    fsx::write(&gradle, &format!("{text}\n\n{block}"))
}

/// Resolve keystore credentials from the environment.
pub fn keystore_credentials(
    signing: Signing,
    store_env: &str,
    key_env: &str,
) -> Result<(String, String)> {
    if signing != Signing::Keystore {
        return Ok((String::new(), String::new()));
    }
    let store = std::env::var(store_env).map_err(|_| {
        Error::with_hint(
            format!("environment variable `{store_env}` is not set"),
            format!("Export the keystore password: export {store_env}='...'"),
        )
    })?;
    let key = std::env::var(key_env).unwrap_or_else(|_| store.clone());
    Ok((store, key))
}

/// Find the produced APK for a variant.
pub fn find_apk(workspace: &Workspace, build_type: &str) -> Result<PathBuf> {
    let dir = workspace.apk_dir(build_type);
    if !dir.is_dir() {
        return Err(Error::with_hint(
            format!("no APK output directory at {}", dir.display()),
            "The Gradle build reported success but produced no artifacts.",
        ));
    }
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|err| Error::io(format!("could not read `{}`", dir.display()), err))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .map(|ext| ext.eq_ignore_ascii_case("apk"))
                .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    // Prefer a signed artifact over an -unsigned one.
    candidates.sort_by_key(|path| {
        path.file_name()
            .map(|name| name.to_string_lossy().contains("unsigned"))
            .unwrap_or(false)
    });
    candidates.into_iter().next().ok_or_else(|| {
        Error::with_hint(
            format!("no .apk file found in {}", dir.display()),
            "Check the build log for Gradle warnings.",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_gradle_numbers() {
        let text = "ext {\n    minSdkVersion = 22\n    targetSdkVersion = 33\n}";
        let out = replace_gradle_number(text, "minSdkVersion", 24);
        assert!(out.contains("minSdkVersion = 24"));
        assert!(out.contains("targetSdkVersion = 33"));
    }

    #[test]
    fn replaces_xml_strings() {
        let text = "<resources>\n  <string name=\"app_name\">old</string>\n</resources>";
        let out = replace_xml_string(text, "app_name", "New Name");
        assert!(out.contains("<string name=\"app_name\">New Name</string>"));
    }

    #[test]
    fn escapes_xml_entities() {
        assert_eq!(escape_xml("Tom & Jerry"), "Tom &amp; Jerry");
    }
}

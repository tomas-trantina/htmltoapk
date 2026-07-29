//! Configuration: global defaults persisted as TOML in `~/.config/htmltoapk/config.toml`.
//!
//! The struct is the single source of truth. `get`/`set` operate on dotted,
//! camelCase keys (`appIdPrefix`, `keystore.path`, `android.minSdk`) by routing
//! through `serde_json`, which means new fields become configurable for free in
//! both the CLI and the TUI config editor.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::{fsx, naming, paths};
use crate::error::{Error, Result};

/// Gradle build variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildType {
    Debug,
    Release,
}

impl BuildType {
    pub fn as_str(self) -> &'static str {
        match self {
            BuildType::Debug => "debug",
            BuildType::Release => "release",
        }
    }

    pub fn gradle_task(self) -> &'static str {
        match self {
            BuildType::Debug => "assembleDebug",
            BuildType::Release => "assembleRelease",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "debug" | "d" => Ok(BuildType::Debug),
            "release" | "r" => Ok(BuildType::Release),
            other => Err(Error::config(format!(
                "unknown build type `{other}` (expected `debug` or `release`)"
            ))),
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            BuildType::Debug => BuildType::Release,
            BuildType::Release => BuildType::Debug,
        }
    }
}

impl Default for BuildType {
    fn default() -> Self {
        BuildType::Debug
    }
}

impl fmt::Display for BuildType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How release APKs should be signed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Signing {
    /// Leave the APK unsigned (release builds produce `-unsigned.apk`).
    None,
    /// Use the Android debug keystore (default, works out of the box).
    Debug,
    /// Use the keystore configured in `[keystore]`.
    Keystore,
}

impl Signing {
    pub fn as_str(self) -> &'static str {
        match self {
            Signing::None => "none",
            Signing::Debug => "debug",
            Signing::Keystore => "keystore",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "none" | "unsigned" => Ok(Signing::None),
            "debug" => Ok(Signing::Debug),
            "keystore" | "release" => Ok(Signing::Keystore),
            other => Err(Error::config(format!(
                "unknown signing mode `{other}` (expected `none`, `debug` or `keystore`)"
            ))),
        }
    }

    pub fn cycled(self) -> Self {
        match self {
            Signing::None => Signing::Debug,
            Signing::Debug => Signing::Keystore,
            Signing::Keystore => Signing::None,
        }
    }
}

impl Default for Signing {
    fn default() -> Self {
        Signing::Debug
    }
}

impl fmt::Display for Signing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Keystore used when `signing = "keystore"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Keystore {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    /// Environment variable holding the store password.
    pub store_password_env: String,
    /// Environment variable holding the key password.
    pub key_password_env: String,
}

impl Default for Keystore {
    fn default() -> Self {
        Keystore {
            path: None,
            alias: None,
            store_password_env: "HTMLTOAPK_STORE_PASSWORD".to_string(),
            key_password_env: "HTMLTOAPK_KEY_PASSWORD".to_string(),
        }
    }
}

/// Android platform knobs written into the generated Gradle project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AndroidConfig {
    pub min_sdk: u32,
    pub target_sdk: u32,
    pub compile_sdk: u32,
    pub version_name: String,
    pub version_code: u32,
}

impl Default for AndroidConfig {
    fn default() -> Self {
        AndroidConfig {
            min_sdk: 23,
            target_sdk: 34,
            compile_sdk: 34,
            version_name: "1.0.0".to_string(),
            version_code: 1,
        }
    }
}

/// Capacitor project knobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CapacitorConfig {
    /// Major version range used for `@capacitor/*` dependencies.
    pub version: String,
    pub web_dir: String,
    pub android_scheme: String,
    pub allow_mixed_content: bool,
    /// Extra npm packages installed into every workspace (Capacitor plugins).
    pub plugins: Vec<String>,
}

impl Default for CapacitorConfig {
    fn default() -> Self {
        CapacitorConfig {
            version: "6".to_string(),
            web_dir: "www".to_string(),
            android_scheme: "https".to_string(),
            allow_mixed_content: true,
            plugins: Vec::new(),
        }
    }
}

/// Icon / splash defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AssetsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub splash: Option<PathBuf>,
    pub background_color: String,
    /// Run `@capacitor/assets` to generate every density from the source images.
    pub generate: bool,
}

impl Default for AssetsConfig {
    fn default() -> Self {
        AssetsConfig {
            icon: None,
            splash: None,
            background_color: "#0B0F17".to_string(),
            generate: true,
        }
    }
}

/// npm behaviour.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NpmConfig {
    pub offline: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    pub extra_args: Vec<String>,
}

impl Default for NpmConfig {
    fn default() -> Self {
        NpmConfig {
            offline: false,
            registry: None,
            extra_args: Vec::new(),
        }
    }
}

/// Global defaults for every build.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    /// Package prefix, e.g. `com.user`. The app segment is appended automatically.
    pub app_id_prefix: String,
    /// Fallback application name when nothing can be derived and nothing is passed.
    pub app_name: String,
    pub build_type: BuildType,
    /// Root directory for generated Capacitor workspaces.
    pub workspace: PathBuf,
    /// Default directory for produced APKs.
    pub output_dir: PathBuf,
    /// Keep the generated workspace after a build (much faster rebuilds).
    pub keep_workspace: bool,
    /// Derive app name + application id from the input path automatically.
    pub auto_naming: bool,
    pub signing: Signing,
    pub keystore: Keystore,
    pub assets: AssetsConfig,
    pub android: AndroidConfig,
    pub capacitor: CapacitorConfig,
    pub npm: NpmConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            app_id_prefix: "com.example".to_string(),
            app_name: "My App".to_string(),
            build_type: BuildType::default(),
            workspace: paths::default_workspace_root(),
            output_dir: PathBuf::from("."),
            keep_workspace: true,
            auto_naming: true,
            signing: Signing::default(),
            keystore: Keystore::default(),
            assets: AssetsConfig::default(),
            android: AndroidConfig::default(),
            capacitor: CapacitorConfig::default(),
            npm: NpmConfig::default(),
        }
    }
}

/// Every settable key, in display order. Also drives the TUI config editor.
pub const KEYS: &[&str] = &[
    "appIdPrefix",
    "appName",
    "buildType",
    "workspace",
    "outputDir",
    "keepWorkspace",
    "autoNaming",
    "signing",
    "keystore.path",
    "keystore.alias",
    "keystore.storePasswordEnv",
    "keystore.keyPasswordEnv",
    "assets.icon",
    "assets.splash",
    "assets.backgroundColor",
    "assets.generate",
    "android.minSdk",
    "android.targetSdk",
    "android.compileSdk",
    "android.versionName",
    "android.versionCode",
    "capacitor.version",
    "capacitor.webDir",
    "capacitor.androidScheme",
    "capacitor.allowMixedContent",
    "capacitor.plugins",
    "npm.offline",
    "npm.registry",
    "npm.extraArgs",
];

/// Keys that accept an empty value (cleared to `null`).
const NULLABLE_KEYS: &[&str] = &[
    "keystore.path",
    "keystore.alias",
    "assets.icon",
    "assets.splash",
    "npm.registry",
];

/// Short help shown next to each key in the TUI.
pub fn describe(key: &str) -> &'static str {
    match key {
        "appIdPrefix" => "Package prefix, e.g. com.user (app segment is appended)",
        "appName" => "Fallback application name",
        "buildType" => "debug | release",
        "workspace" => "Where Capacitor workspaces are generated",
        "outputDir" => "Default directory for produced APKs",
        "keepWorkspace" => "Keep workspaces after a build (faster rebuilds)",
        "autoNaming" => "Derive app name and application id from the input path",
        "signing" => "none | debug | keystore",
        "keystore.path" => "Path to a .jks / .keystore file",
        "keystore.alias" => "Key alias inside the keystore",
        "keystore.storePasswordEnv" => "Env var holding the store password",
        "keystore.keyPasswordEnv" => "Env var holding the key password",
        "assets.icon" => "Source icon (1024x1024 PNG recommended)",
        "assets.splash" => "Source splash image (2732x2732 PNG recommended)",
        "assets.backgroundColor" => "Splash background colour (#RRGGBB)",
        "assets.generate" => "Generate densities with @capacitor/assets",
        "android.minSdk" => "Minimum supported Android SDK level",
        "android.targetSdk" => "Target Android SDK level",
        "android.compileSdk" => "Compile Android SDK level",
        "android.versionName" => "versionName written to the manifest",
        "android.versionCode" => "versionCode written to the manifest",
        "capacitor.version" => "Capacitor major version range",
        "capacitor.webDir" => "Web asset directory inside the workspace",
        "capacitor.androidScheme" => "WebView scheme (https recommended)",
        "capacitor.allowMixedContent" => "Allow http content inside the WebView",
        "capacitor.plugins" => "Comma separated npm packages to install",
        "npm.offline" => "Pass --offline to npm install",
        "npm.registry" => "Custom npm registry URL",
        "npm.extraArgs" => "Comma separated extra npm install arguments",
        _ => "",
    }
}

impl Config {
    /// Path of the configuration file.
    pub fn path() -> Result<PathBuf> {
        paths::config_file()
    }

    /// Does a configuration file exist already?
    pub fn exists() -> bool {
        Config::path().map(|path| path.is_file()).unwrap_or(false)
    }

    /// Load the configuration, falling back to defaults when no file exists.
    pub fn load() -> Result<Self> {
        let path = Config::path()?;
        if path.is_file() {
            Config::load_from(&path)
        } else {
            Ok(Config::default())
        }
    }

    /// Load the configuration from an explicit path.
    pub fn load_from(path: &Path) -> Result<Self> {
        let text = fsx::read_to_string(path)?;
        let config: Config = toml::from_str(&text).map_err(|err| Error::InvalidConfig {
            path: Some(path.to_path_buf()),
            reason: err.message().to_string(),
        })?;
        config.validate().map_err(|err| match err {
            Error::InvalidConfig { reason, .. } => Error::InvalidConfig {
                path: Some(path.to_path_buf()),
                reason,
            },
            other => other,
        })?;
        Ok(config)
    }

    /// Persist to the default location, returning the written path.
    pub fn save(&self) -> Result<PathBuf> {
        let path = Config::path()?;
        self.save_to(&path)?;
        Ok(path)
    }

    /// Persist to an explicit path.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        self.validate()?;
        fsx::write(path, &self.to_toml()?)
    }

    /// Render as commented TOML.
    pub fn to_toml(&self) -> Result<String> {
        let body = toml::to_string_pretty(self)
            .map_err(|err| Error::config(format!("could not serialize configuration: {err}")))?;
        Ok(format!(
            "# htmltoapk configuration\n\
             # Docs: https://github.com/yourname/htmltoapk#configuration\n\
             # Edit by hand, or use: htmltoapk config set <key> <value>\n\n{body}"
        ))
    }

    /// Validate every invariant that does not require touching the filesystem.
    pub fn validate(&self) -> Result<()> {
        validate_app_id_prefix(&self.app_id_prefix)?;
        if self.app_name.trim().is_empty() {
            return Err(Error::config("`appName` must not be empty"));
        }
        if self.workspace.as_os_str().is_empty() {
            return Err(Error::config("`workspace` must not be empty"));
        }
        if self.output_dir.as_os_str().is_empty() {
            return Err(Error::config("`outputDir` must not be empty"));
        }
        if self.android.min_sdk < 21 {
            return Err(Error::config(
                "`android.minSdk` must be at least 21 (Capacitor requirement)",
            ));
        }
        if self.android.target_sdk < self.android.min_sdk {
            return Err(Error::config(
                "`android.targetSdk` must be greater than or equal to `android.minSdk`",
            ));
        }
        if self.android.compile_sdk < self.android.target_sdk {
            return Err(Error::config(
                "`android.compileSdk` must be greater than or equal to `android.targetSdk`",
            ));
        }
        if self.android.version_name.trim().is_empty() {
            return Err(Error::config("`android.versionName` must not be empty"));
        }
        if self.android.version_code == 0 {
            return Err(Error::config("`android.versionCode` must be at least 1"));
        }
        if self.capacitor.version.trim().is_empty() {
            return Err(Error::config("`capacitor.version` must not be empty"));
        }
        if self.capacitor.web_dir.trim().is_empty() {
            return Err(Error::config("`capacitor.webDir` must not be empty"));
        }
        if !self.assets.background_color.starts_with('#') {
            return Err(Error::config(
                "`assets.backgroundColor` must be a hex colour such as #0B0F17",
            ));
        }
        Ok(())
    }

    /// Read a single dotted key as text.
    pub fn get(&self, key: &str) -> Result<String> {
        let key = normalize_key(key);
        if !KEYS.contains(&key.as_str()) {
            return Err(unknown_key(&key));
        }
        let json = self.to_json()?;
        Ok(lookup(&json, &key).map(render_value).unwrap_or_default())
    }

    /// Write a single dotted key from text, validating the result.
    pub fn set(&mut self, key: &str, raw: &str) -> Result<()> {
        let key = normalize_key(key);
        if !KEYS.contains(&key.as_str()) {
            return Err(unknown_key(&key));
        }
        let mut json = self.to_json()?;
        let segments: Vec<&str> = key.split('.').collect();
        let (last, parents) = segments
            .split_last()
            .ok_or_else(|| unknown_key(&key))?;

        let mut cursor = &mut json;
        for segment in parents {
            cursor = cursor
                .get_mut(*segment)
                .ok_or_else(|| unknown_key(&key))?;
        }
        let object = cursor
            .as_object_mut()
            .ok_or_else(|| unknown_key(&key))?;
        let previous = object.get(*last).cloned().unwrap_or(Value::Null);
        let value = coerce(&key, &previous, raw)?;
        object.insert((*last).to_string(), value);

        let updated: Config = serde_json::from_value(json).map_err(|err| {
            Error::config(format!("cannot set `{key}` to `{}`: {err}", raw.trim()))
        })?;
        updated.validate()?;
        *self = updated;
        Ok(())
    }

    /// All keys with their current values, for `htmltoapk config` and the TUI.
    pub fn entries(&self) -> Vec<(&'static str, String)> {
        KEYS.iter()
            .map(|key| (*key, self.get(key).unwrap_or_default()))
            .collect()
    }

    fn to_json(&self) -> Result<Value> {
        serde_json::to_value(self)
            .map_err(|err| Error::config(format!("could not serialize configuration: {err}")))
    }

    /// Workspace root with `~` expanded.
    pub fn workspace_root(&self) -> PathBuf {
        paths::expand_tilde(&self.workspace)
    }

    /// Output directory with `~` expanded.
    pub fn output_root(&self) -> PathBuf {
        paths::expand_tilde(&self.output_dir)
    }

    /// Application name: explicit value wins, then auto-naming, then the default.
    pub fn resolve_app_name(&self, explicit: Option<&str>, derived: Option<&str>) -> String {
        if let Some(name) = explicit {
            let name = name.trim();
            if !name.is_empty() {
                return name.to_string();
            }
        }
        if self.auto_naming {
            if let Some(name) = derived {
                let name = name.trim();
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
        self.app_name.clone()
    }

    /// Application id: explicit value wins, otherwise `appIdPrefix` + app segment.
    pub fn resolve_app_id(&self, explicit: Option<&str>, app_name: &str) -> Result<String> {
        if let Some(id) = explicit {
            let id = id.trim();
            if !id.is_empty() {
                if !naming::is_valid_app_id(id) {
                    return Err(Error::with_hint(
                        format!("`{id}` is not a valid Android application id"),
                        "Use lowercase, dot separated segments, e.g. com.user.myapp",
                    ));
                }
                return Ok(id.to_string());
            }
        }
        Ok(naming::app_id(&self.app_id_prefix, app_name))
    }
}

/// Validate an `appIdPrefix` such as `com.user`.
pub fn validate_app_id_prefix(prefix: &str) -> Result<()> {
    let prefix = prefix.trim();
    let segments: Vec<&str> = prefix.split('.').collect();
    if prefix.is_empty() || segments.len() < 2 {
        return Err(Error::with_hint(
            format!("`appIdPrefix` must contain at least two segments (got `{prefix}`)"),
            "Example: htmltoapk config set appIdPrefix com.user",
        ));
    }
    for segment in segments {
        if segment.is_empty() {
            return Err(Error::config(
                "`appIdPrefix` must not contain empty segments",
            ));
        }
        if !segment
            .chars()
            .next()
            .map(|ch| ch.is_ascii_lowercase())
            .unwrap_or(false)
        {
            return Err(Error::config(format!(
                "`appIdPrefix` segment `{segment}` must start with a lowercase letter"
            )));
        }
        if !segment
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
        {
            return Err(Error::config(format!(
                "`appIdPrefix` segment `{segment}` may only contain a-z, 0-9 and _"
            )));
        }
        if naming::is_reserved(segment) {
            return Err(Error::config(format!(
                "`appIdPrefix` segment `{segment}` is a reserved Java/Kotlin keyword"
            )));
        }
    }
    Ok(())
}

fn unknown_key(key: &str) -> Error {
    Error::with_hint(
        format!("unknown configuration key `{key}`"),
        format!("Available keys:\n  {}", KEYS.join("\n  ")),
    )
}

/// Accept `app_id_prefix`, `app-id-prefix` and `appIdPrefix` for the same key.
fn normalize_key(key: &str) -> String {
    key.trim()
        .split('.')
        .map(to_camel_case)
        .collect::<Vec<String>>()
        .join(".")
}

fn to_camel_case(segment: &str) -> String {
    let mut out = String::new();
    let mut upper_next = false;
    for ch in segment.chars() {
        if ch == '_' || ch == '-' {
            upper_next = true;
            continue;
        }
        if upper_next {
            out.extend(ch.to_uppercase());
            upper_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn lookup<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    let mut cursor = value;
    for segment in key.split('.') {
        cursor = cursor.get(segment)?;
    }
    Some(cursor)
}

fn render_value(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .map(render_value)
            .collect::<Vec<String>>()
            .join(","),
        other => other.to_string(),
    }
}

/// Convert user text into the JSON type currently stored at `key`.
fn coerce(key: &str, previous: &Value, raw: &str) -> Result<Value> {
    let trimmed = raw.trim();
    let cleared = trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") || trimmed == "-";

    if NULLABLE_KEYS.contains(&key) && cleared {
        return Ok(Value::Null);
    }

    match previous {
        Value::Bool(_) => match trimmed.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "y" | "on" => Ok(Value::Bool(true)),
            "0" | "false" | "no" | "n" | "off" => Ok(Value::Bool(false)),
            other => Err(Error::config(format!(
                "`{other}` is not a boolean (use true or false)"
            ))),
        },
        Value::Number(_) => {
            let number: i64 = trimmed
                .parse()
                .map_err(|_| Error::config(format!("`{trimmed}` is not a whole number")))?;
            Ok(Value::Number(serde_json::Number::from(number)))
        }
        Value::Array(_) => Ok(Value::Array(
            trimmed
                .split(',')
                .map(|item| item.trim())
                .filter(|item| !item.is_empty())
                .map(|item| Value::String(item.to_string()))
                .collect(),
        )),
        Value::Object(_) => Err(Error::config(format!(
            "`{key}` is a section and cannot be set directly"
        ))),
        Value::String(_) | Value::Null => Ok(Value::String(trimmed.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        Config::default().validate().expect("defaults must validate");
    }

    #[test]
    fn round_trips_through_toml() {
        let config = Config::default();
        let text = config.to_toml().unwrap();
        let parsed: Config = toml::from_str(&text).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn every_key_is_readable() {
        let config = Config::default();
        for key in KEYS {
            config.get(key).unwrap_or_else(|_| panic!("missing key {key}"));
        }
    }

    #[test]
    fn set_coerces_types() {
        let mut config = Config::default();
        config.set("appIdPrefix", "com.user").unwrap();
        config.set("keep_workspace", "false").unwrap();
        config.set("android.minSdk", "24").unwrap();
        config.set("capacitor.plugins", "@capacitor/app, @capacitor/haptics").unwrap();
        config.set("keystore.path", "").unwrap();
        assert_eq!(config.app_id_prefix, "com.user");
        assert!(!config.keep_workspace);
        assert_eq!(config.android.min_sdk, 24);
        assert_eq!(config.capacitor.plugins.len(), 2);
        assert!(config.keystore.path.is_none());
    }

    #[test]
    fn set_rejects_invalid_values() {
        let mut config = Config::default();
        assert!(config.set("appIdPrefix", "nodots").is_err());
        assert!(config.set("buildType", "turbo").is_err());
        assert!(config.set("android.minSdk", "nope").is_err());
        assert!(config.set("nope", "1").is_err());
    }

    #[test]
    fn resolves_names_and_ids() {
        let mut config = Config::default();
        config.app_id_prefix = "com.user".to_string();
        let name = config.resolve_app_name(None, Some("Kratom Tracker"));
        assert_eq!(name, "Kratom Tracker");
        assert_eq!(
            config.resolve_app_id(None, &name).unwrap(),
            "com.user.kratomtracker"
        );
        assert!(config.resolve_app_id(Some("Bad.Id"), &name).is_err());
    }
}

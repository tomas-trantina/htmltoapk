//! Smart naming: derive a human app name, a valid Android package id and a
//! sensible APK file name from an arbitrary input path.

use std::path::Path;

/// Java / Kotlin reserved words that must never appear as a package segment.
const RESERVED: &[&str] = &[
    "abstract", "assert", "boolean", "break", "byte", "case", "catch", "char", "class", "const",
    "continue", "default", "do", "double", "else", "enum", "extends", "final", "finally", "float",
    "for", "goto", "if", "implements", "import", "instanceof", "int", "interface", "long",
    "native", "new", "package", "private", "protected", "public", "return", "short", "static",
    "strictfp", "super", "switch", "synchronized", "this", "throw", "throws", "transient", "try",
    "void", "volatile", "while", "true", "false", "null", "fun", "val", "var", "object", "when",
    "in", "is", "typealias",
];

/// Is `word` a reserved Java/Kotlin keyword?
pub fn is_reserved(word: &str) -> bool {
    RESERVED.contains(&word)
}

/// `My Cool App!` -> `my-cool-app`
pub fn slug(input: &str) -> String {
    let mut out = String::new();
    let mut previous_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.extend(ch.to_lowercase());
            previous_dash = false;
        } else if !out.is_empty() && !previous_dash {
            out.push('-');
            previous_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "app".to_string()
    } else {
        trimmed
    }
}

/// `my_cool-app.html` -> `My Cool App`
pub fn humanize(input: &str) -> String {
    let words: Vec<String> = input
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .flat_map(split_camel_case)
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    let rest: String = chars.collect();
                    let rest = if rest.chars().all(|c| c.is_ascii_uppercase()) && rest.len() > 1 {
                        rest.to_lowercase()
                    } else {
                        rest
                    };
                    format!("{}{}", first.to_uppercase(), rest)
                }
                None => String::new(),
            }
        })
        .filter(|word| !word.is_empty())
        .collect();

    if words.is_empty() {
        "My App".to_string()
    } else {
        words.join(" ")
    }
}

fn split_camel_case(word: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = word.chars().collect();
    for (index, ch) in chars.iter().enumerate() {
        let previous_lower = index > 0 && chars[index - 1].is_ascii_lowercase();
        let next_lower = index + 1 < chars.len() && chars[index + 1].is_ascii_lowercase();
        if ch.is_ascii_uppercase() && !current.is_empty() && (previous_lower || next_lower) {
            parts.push(std::mem::take(&mut current));
        }
        current.push(*ch);
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

/// Derive a display name from a file or directory path.
pub fn app_name_from_path(path: &Path) -> String {
    let raw = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_default();
    let raw = if raw.is_empty() || raw == "." || raw == ".." {
        std::env::current_dir()
            .ok()
            .and_then(|dir| dir.file_name().map(|name| name.to_string_lossy().to_string()))
            .unwrap_or_else(|| "My App".to_string())
    } else {
        raw
    };
    // `index.html` inside `awesome-site/` should become "Awesome Site".
    if raw.eq_ignore_ascii_case("index") || raw.eq_ignore_ascii_case("main") {
        if let Some(parent) = path.parent().and_then(|parent| parent.file_name()) {
            let parent = parent.to_string_lossy().to_string();
            if !parent.is_empty() && parent != "." {
                return humanize(&parent);
            }
        }
    }
    humanize(&raw)
}

/// Turn an app name into a single, valid package segment: `My App 2` -> `myapp2`.
pub fn package_segment(app_name: &str) -> String {
    let mut segment: String = app_name
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect();
    if segment.is_empty() {
        segment = "app".to_string();
    }
    if segment.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(true) || is_reserved(&segment) {
        segment = format!("app{segment}");
    }
    segment
}

/// `com.user` + `My App` -> `com.user.myapp`
pub fn app_id(prefix: &str, app_name: &str) -> String {
    let prefix = prefix.trim().trim_matches('.');
    let segment = package_segment(app_name);
    if prefix.is_empty() {
        format!("com.htmltoapk.{segment}")
    } else {
        format!("{prefix}.{segment}")
    }
}

/// Is `candidate` already a fully-qualified application id?
pub fn is_valid_app_id(candidate: &str) -> bool {
    let segments: Vec<&str> = candidate.split('.').collect();
    if segments.len() < 2 {
        return false;
    }
    segments.iter().all(|segment| {
        !segment.is_empty()
            && segment.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false)
            && segment
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            && !is_reserved(segment)
    })
}

/// Default APK file name: `my-app-debug.apk`
pub fn apk_file_name(app_name: &str, build_type: &str) -> String {
    format!("{}-{}.apk", slug(app_name), build_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn slugs_are_clean() {
        assert_eq!(slug("My Cool App!"), "my-cool-app");
        assert_eq!(slug("---"), "app");
    }

    #[test]
    fn humanize_handles_separators_and_camel_case() {
        assert_eq!(humanize("my_cool-app"), "My Cool App");
        assert_eq!(humanize("kratomTracker"), "Kratom Tracker");
    }

    #[test]
    fn index_html_uses_parent_directory() {
        let path = PathBuf::from("awesome-site/index.html");
        assert_eq!(app_name_from_path(&path), "Awesome Site");
    }

    #[test]
    fn package_segments_are_valid() {
        assert_eq!(package_segment("My App 2"), "myapp2");
        assert_eq!(package_segment("2048"), "app2048");
        assert_eq!(package_segment("class"), "appclass");
    }

    #[test]
    fn app_ids_are_validated() {
        assert!(is_valid_app_id("com.user.myapp"));
        assert!(!is_valid_app_id("myapp"));
        assert!(!is_valid_app_id("com.User.myapp"));
        assert_eq!(app_id("com.user", "My App"), "com.user.myapp");
    }

    #[test]
    fn apk_names_include_build_type() {
        assert_eq!(apk_file_name("My App", "debug"), "my-app-debug.apk");
    }
}

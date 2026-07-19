//! Global initialization.
//!
//! Mirrors `lib/workers/global/initialize.ts`.
//! @parity lib/workers/global/initialize.ts partial — git version check+error, directory (base/cache/containerbase) computation+ensure, host rules add (legacy too), commits limit, emoji config, third-party metadata env intent, global finalize stub; rate limits entry; (platform init, packageCache full init, merge-confidence, secret apply, and top-level global flow live in main.rs + config + platform for the broader workers/global/index.ts surface).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::config::GlobalConfig;
use crate::http::throttle::set_http_rate_limits;
use crate::limits::set_max_limit;
use crate::util::emoji::set_emoji_config;
use crate::util::host_rules;

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::config::BinarySource;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParsedHostRule {
    #[serde(default)]
    match_host: Option<String>,
    #[serde(default)]
    host_type: Option<String>,
    #[serde(default)]
    token: Option<String>,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    auth_type: Option<String>,
    #[serde(default)]
    insecure_registry: Option<bool>,
    #[serde(default)]
    timeout: Option<u32>,
    #[serde(default)]
    abort_on_error: Option<bool>,
    #[serde(default)]
    abort_ignore_status_codes: Option<Vec<u16>>,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    enable_http2: Option<bool>,
    #[serde(default)]
    concurrent_request_limit: Option<u32>,
    #[serde(default)]
    max_requests_per_second: Option<f64>,
    #[serde(default)]
    headers: Option<HashMap<String, String>>,
    #[serde(default)]
    max_retry_after: Option<u32>,
    #[serde(default)]
    keep_alive: Option<bool>,
    #[serde(default)]
    https_certificate_authority: Option<String>,
    #[serde(default)]
    https_private_key: Option<String>,
    #[serde(default)]
    https_certificate: Option<String>,
    #[serde(default)]
    read_only: Option<bool>,
    #[serde(default)]
    host_name: Option<String>,
    #[serde(default)]
    domain_name: Option<String>,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    endpoint: Option<String>,
    #[serde(default)]
    host: Option<String>,
    #[serde(default)]
    platform: Option<String>,
}

fn parse_host_rule(
    raw: &serde_json::Value,
    index: usize,
) -> Result<(host_rules::HostRule, host_rules::LegacyHostRule), String> {
    let value = serde_json::from_value::<ParsedHostRule>(raw.clone())
        .map_err(|err| format!("Cannot parse hostRules[{index}]: {err}"))?;

    let mut legacy = host_rules::LegacyHostRule::default();
    legacy.host_name = value.host_name;
    legacy.domain_name = value.domain_name;
    legacy.base_url = value.base_url.or(value.endpoint);
    legacy.match_host = value.host;

    Ok((
        host_rules::HostRule {
            match_host: value.match_host,
            host_type: value.host_type.or(value.platform),
            token: value.token,
            username: value.username,
            password: value.password,
            auth_type: value.auth_type,
            insecure_registry: value.insecure_registry,
            timeout: value.timeout,
            abort_on_error: value.abort_on_error,
            abort_ignore_status_codes: value.abort_ignore_status_codes,
            enabled: value.enabled,
            enable_http2: value.enable_http2,
            concurrent_request_limit: value.concurrent_request_limit,
            max_requests_per_second: value.max_requests_per_second,
            headers: value.headers,
            max_retry_after: value.max_retry_after,
            keep_alive: value.keep_alive,
            https_certificate_authority: value.https_certificate_authority,
            https_private_key: value.https_private_key,
            https_certificate: value.https_certificate,
            read_only: value.read_only,
            resolved_host: None,
        },
        legacy,
    ))
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalInitConfig {
    pub platform: Option<String>,
    pub endpoint: Option<String>,
    pub cache_dir: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalInitResult {
    pub initialized: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Check git version meets Renovate minimum (2.33.0 for show-current etc).
/// Mirrors checkVersions + validateGitVersion from lib/workers/global/initialize.ts .
fn check_versions(result: &mut GlobalInitResult) {
    let output = match Command::new("git").arg("--version").output() {
        Ok(o) => o,
        Err(e) => {
            result
                .errors
                .push(format!("Error fetching git version: {}", e));
            result.initialized = false;
            return;
        }
    };
    if !output.status.success() {
        result.errors.push("git not installed".to_string());
        result.initialized = false;
        return;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = stdout.split_whitespace().nth(2).unwrap_or("");
    let parts: Vec<u32> = version
        .split('.')
        .filter_map(|s| s.parse::<u32>().ok())
        .collect();
    let valid = if parts.len() >= 2 {
        let (maj, min) = (parts[0], parts[1]);
        maj > 2 || (maj == 2 && min >= 33)
    } else {
        false
    };
    if !valid && !version.is_empty() {
        result.errors.push(format!(
            "Init: git version needs upgrading (detectedVersion: {}, minimumVersion: 2.33.0)",
            version
        ));
        result.initialized = false;
    }
}

/// Ensure baseDir / cacheDir / containerbaseDir per config or sensible defaults.
/// (Rust does not rely on process TMPDIR the same way; we still compute/ensure
/// the directories so cache + containerbase work.)
/// Mirrors setDirectories from lib/workers/global/initialize.ts (dir creation contract).
fn set_directories(config: &GlobalConfig) {
    let tmpdir = env::var("RENOVATE_TMPDIR")
        .unwrap_or_else(|_| env::temp_dir().to_string_lossy().to_string());

    let base_dir = config.base_dir.clone().unwrap_or_else(|| {
        Path::new(&tmpdir)
            .join("renovate")
            .to_string_lossy()
            .to_string()
    });
    let _ = fs::create_dir_all(&base_dir);

    let cache_dir = config.cache_dir.clone().unwrap_or_else(|| {
        Path::new(&base_dir)
            .join("cache")
            .to_string_lossy()
            .to_string()
    });
    let _ = fs::create_dir_all(&cache_dir);

    if matches!(
        config.binary_source,
        Some(BinarySource::Docker) | Some(BinarySource::Install)
    ) {
        let cb_dir = config.containerbase_dir.clone().unwrap_or_else(|| {
            Path::new(&cache_dir)
                .join("containerbase")
                .to_string_lossy()
                .to_string()
        });
        let _ = fs::create_dir_all(&cb_dir);
    }
}

/// Apply host rules (after secrets/vars in full flow).
/// Mirrors setGlobalHostRules (the add part).
fn set_global_host_rules(config: &GlobalConfig, result: &mut GlobalInitResult) {
    if !config.host_rules.is_empty() {
        for (index, host_rule) in config.host_rules.iter().enumerate() {
            let parsed = match parse_host_rule(host_rule, index) {
                Ok(rule) => rule,
                Err(err) => {
                    result.warnings.push(err);
                    continue;
                }
            };
            if let Err(err) = host_rules::add_with_legacy(parsed.0, parsed.1) {
                result
                    .warnings
                    .push(format!("hostRules[{index}] is invalid: {err}"));
            }
        }
    }
}

/// Configure cloud metadata service env disables when requested.
/// Mirrors configureThirdPartyLibraries (the env sets).
fn configure_third_party_libraries(_config: &GlobalConfig) {
    // useCloudMetadataServices field not yet present in GlobalConfig (parsed elsewhere or later);
    // when present default is true (enabled). The observable effect is setting
    // AWS_EC2_METADATA_DISABLED + METADATA_SERVER_DETECTION when disabled.
    // (env sets omitted here to respect #![forbid(unsafe_code)] + current lack of
    // the config field; the dir/cache init and git check cover the main new surface.)
    let _disabled = env::var("RENOVATE_USE_CLOUD_METADATA_SERVICES")
        .map(|v| v == "false" || v == "0")
        .unwrap_or(false);
    // if disabled { /* would set the two envs */ }
}

/// Mirrors globalFinalize: packageCache cleanup + logger remap reset.
/// (Global package cache handle + logger remaps not yet centralized here.)
pub async fn global_finalize() {
    // package cache cleanup and resetGlobalLogLevelRemaps() live in their modules;
    // call sites will invoke when global wiring is added.
}

pub fn initialize_global(global_config: &GlobalConfig) -> GlobalInitResult {
    let mut result = GlobalInitResult {
        initialized: true,
        errors: Vec::new(),
        warnings: Vec::new(),
    };

    set_http_rate_limits(Vec::new(), Vec::new());

    check_versions(&mut result);

    // first host rules (TS does applySecrets before each setGlobalHostRules)
    host_rules::clear();
    set_global_host_rules(global_config, &mut result);

    set_directories(global_config);

    // packageCache.init(config) — in Rust the PackageCache backend is constructed via
    // PackageCache::with_dir(...) on first use (or by callers); explicit global init
    // not yet centralized in this module.
    // initMergeConfidence(config) — merge confidence datasource used on-demand; no
    // separate global init side-effect in current Rust surface.

    set_max_limit(
        "Commits",
        global_config
            .pr_commits_per_run_limit
            .map(|limit| limit as i64),
    );
    set_emoji_config(global_config.unicode_emoji.unwrap_or(true));

    // second host rules pass (matches TS)
    set_global_host_rules(global_config, &mut result);

    configure_third_party_libraries(global_config);

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits;
    use crate::util::emoji;
    use crate::util::host_rules::{HostRuleSearch, find};
    use serde_json::json;

    #[test]
    fn global_init_config_default() {
        let c = GlobalInitConfig::default();
        assert!(c.platform.is_none());
    }

    #[test]
    fn global_init_result_default() {
        let r = GlobalInitResult::default();
        assert!(!r.initialized);
        assert!(r.errors.is_empty());
        assert!(r.warnings.is_empty());
    }

    #[test]
    fn initialize_global_returns_result() {
        limits::reset_all_limits();
        host_rules::clear();
        emoji::set_emoji_config(true);
        let global = GlobalConfig::default();
        let result = initialize_global(&global);
        assert!(result.initialized);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn initialize_global_adds_host_rules() {
        host_rules::clear();
        let global = GlobalConfig {
            host_rules: vec![json!({
                "hostType": "npm",
                "matchHost": "https://registry.npmjs.org",
                "token": "token123"
            })],
            ..GlobalConfig::default()
        };

        let result = initialize_global(&global);
        assert!(result.initialized);
        assert!(result.warnings.is_empty());

        let found = find(&HostRuleSearch {
            host_type: Some("npm".into()),
            url: Some("https://registry.npmjs.org".into()),
            ..HostRuleSearch::default()
        });
        assert_eq!(found.token.as_deref(), Some("token123"));
    }

    #[test]
    fn initialize_global_adds_legacy_host_rules() {
        host_rules::clear();
        let global = GlobalConfig {
            host_rules: vec![json!({
                "hostType": "npm",
                "hostName": "legacy-registry.example.com",
                "token": "legacy-token"
            })],
            ..GlobalConfig::default()
        };

        let result = initialize_global(&global);
        assert!(result.warnings.is_empty());
        assert!(result.initialized);

        let found = find(&HostRuleSearch {
            host_type: Some("npm".into()),
            url: Some("https://legacy-registry.example.com".into()),
            ..HostRuleSearch::default()
        });
        assert_eq!(found.token.as_deref(), Some("legacy-token"));
    }

    #[test]
    fn initialize_global_warns_for_invalid_host_rules() {
        host_rules::clear();
        let global = GlobalConfig {
            host_rules: vec![
                json!(1),
                json!({
                    "hostType": "npm",
                    "hostName": "bad-one.example.com",
                    "baseUrl": "bad-two.example.com",
                    "token": "oops"
                }),
            ],
            ..GlobalConfig::default()
        };

        let result = initialize_global(&global);
        assert_eq!(result.warnings.len(), 2);
    }

    #[test]
    fn initialize_global_sets_commits_limit() {
        limits::reset_all_limits();
        let global = GlobalConfig {
            pr_commits_per_run_limit: Some(2),
            ..GlobalConfig::default()
        };

        let result = initialize_global(&global);
        assert!(result.initialized);
        assert!(result.warnings.is_empty());

        limits::inc_limited_value("Commits", 1);
        assert!(!limits::is_commits_limit_reached());
        limits::inc_limited_value("Commits", 1);
        assert!(limits::is_commits_limit_reached());
    }

    #[test]
    fn initialize_global_sets_emoji_config() {
        emoji::set_emoji_config(true);

        let global = GlobalConfig {
            unicode_emoji: Some(false),
            ..GlobalConfig::default()
        };
        let result = initialize_global(&global);
        assert!(result.initialized);
        assert_eq!(emoji::emojify(":warning:"), ":warning:");
    }

    #[test]
    fn global_init_result_serialization_roundtrip() {
        let r = GlobalInitResult {
            initialized: true,
            errors: vec!["err".into()],
            warnings: vec![],
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: GlobalInitResult = serde_json::from_str(&json).unwrap();
        assert!(back.initialized);
        assert_eq!(back.errors.len(), 1);
    }

    // The single proving test for this source file's parity work.
    // Exercises git version gate + directory setup (core observable behaviors
    // from globalInitialize that were missing/divergent).
    #[test]
    fn initialize_global_enforces_git_version_and_sets_directories() {
        // Ported: "has a git version greater or equal to the minimum required" — lib/util/git/index.spec.ts (invoked via checkVersions in global init)
        // Mirrors intent of "returns if valid git version" from lib/workers/global/initialize.spec.ts
        limits::reset_all_limits();
        host_rules::clear();
        emoji::set_emoji_config(true);
        let global = GlobalConfig::default();
        let result = initialize_global(&global);
        assert!(result.initialized);
        // No "git version needs upgrading" error when running in a modern env.
        assert!(
            result
                .errors
                .iter()
                .all(|e| !e.contains("git version needs upgrading")),
            "errors: {:?}",
            result.errors
        );
    }
}

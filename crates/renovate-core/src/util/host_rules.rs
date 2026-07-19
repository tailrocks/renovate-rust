//! Host rules management — mirrors `lib/util/host-rules.ts`.
//!
//! Renovate's host-rules system stores credentials and per-host configuration
//! that datasources and HTTP clients use when making requests. Rules are matched
//! by `matchHost` (URL prefix or hostname/domain suffix) and optionally by
//! `hostType` (datasource id) and `readOnly`.

use std::cell::RefCell;

extern crate url as url_lib;

use url_lib::Url;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A host rule as stored internally (after migration from legacy fields).
///
/// Mirrors `HostRule` from `lib/types/host-rules.ts`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HostRule {
    pub auth_type: Option<String>,
    pub token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub insecure_registry: Option<bool>,
    pub timeout: Option<u32>,
    pub abort_on_error: Option<bool>,
    pub abort_ignore_status_codes: Option<Vec<u16>>,
    pub enabled: Option<bool>,
    pub enable_http2: Option<bool>,
    pub concurrent_request_limit: Option<u32>,
    pub max_requests_per_second: Option<f64>,
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub max_retry_after: Option<u32>,
    pub keep_alive: Option<bool>,
    pub https_certificate_authority: Option<String>,
    pub https_private_key: Option<String>,
    pub https_certificate: Option<String>,
    pub host_type: Option<String>,
    pub match_host: Option<String>,
    /// Resolved hostname extracted from `match_host` on insert.
    pub resolved_host: Option<String>,
    pub read_only: Option<bool>,
}

/// Combined result of matching multiple rules, with routing fields stripped.
///
/// Mirrors `CombinedHostRule` from `lib/types/host-rules.ts`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CombinedHostRule {
    pub auth_type: Option<String>,
    pub token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub insecure_registry: Option<bool>,
    pub timeout: Option<u32>,
    pub abort_on_error: Option<bool>,
    pub abort_ignore_status_codes: Option<Vec<u16>>,
    pub enabled: Option<bool>,
    pub enable_http2: Option<bool>,
    pub concurrent_request_limit: Option<u32>,
    pub max_requests_per_second: Option<f64>,
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub max_retry_after: Option<u32>,
    pub keep_alive: Option<bool>,
    pub https_certificate_authority: Option<String>,
    pub https_private_key: Option<String>,
    pub https_certificate: Option<String>,
}

/// Legacy field names accepted by `add()` for backwards compatibility.
#[derive(Debug, Default)]
pub struct LegacyHostRule {
    pub host_name: Option<String>,
    pub domain_name: Option<String>,
    pub base_url: Option<String>,
    pub match_host: Option<String>,
}

/// Search parameters for `find()` / `find_all()`.
#[derive(Debug, Default)]
pub struct HostRuleSearch {
    pub host_type: Option<String>,
    pub url: Option<String>,
    pub read_only: Option<bool>,
}

// ---------------------------------------------------------------------------
// Thread-local state
// ---------------------------------------------------------------------------

thread_local! {
    static HOST_RULES: RefCell<Vec<HostRule>> = const { RefCell::new(Vec::new()) };
}

// ---------------------------------------------------------------------------
// URL helpers — mirrors `lib/util/url.ts`
// ---------------------------------------------------------------------------

/// Normalize a host identifier that lacks a scheme.
///
/// Mirrors `massageHostUrl` from `lib/util/url.ts`.
pub fn massage_host_url(input: &str) -> String {
    if !input.contains("://") && (input.contains('/') || input.contains(':')) {
        format!("https://{input}")
    } else {
        input.to_owned()
    }
}

fn is_http_url(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https")
}

/// Check whether `url` matches `match_host`.
///
/// Mirrors `matchesHost` from `lib/util/host-rules.ts`.
pub fn matches_host(url: &str, match_host: &str) -> bool {
    let Ok(parsed_url) = Url::parse(url) else {
        return false;
    };

    let parsed_match = Url::parse(match_host);
    if is_http_url(&parsed_url)
        && let Ok(ref pmh) = parsed_match
        && is_http_url(pmh)
    {
        // Both are HTTP URLs — prefix match on the full href
        return parsed_url.as_str().starts_with(pmh.as_str());
    }

    // Non-HTTP match_host: compare hostnames
    let hostname = parsed_url.host_str().unwrap_or("");
    if hostname.is_empty() {
        return false;
    }

    if hostname == match_host {
        return true;
    }

    // Subdomain suffix: ".github.com" matches "api.github.com"
    let suffix = if match_host.starts_with('.') {
        match_host.to_owned()
    } else {
        format!(".{match_host}")
    };
    hostname.ends_with(&suffix)
}

// ---------------------------------------------------------------------------
// Rule migration
// ---------------------------------------------------------------------------

/// Migrate legacy host-name fields into `matchHost`, returning a typed rule.
///
/// Mirrors `migrateRule` from `lib/util/host-rules.ts`.
pub fn migrate_rule(mut rule: HostRule, legacy: &LegacyHostRule) -> Result<HostRule, String> {
    let candidates: Vec<&str> = [
        rule.match_host.as_deref(),
        legacy.host_name.as_deref(),
        legacy.domain_name.as_deref(),
        legacy.base_url.as_deref(),
    ]
    .iter()
    .filter_map(|v| *v)
    .collect();

    match candidates.len() {
        0 => {}
        1 => {
            rule.match_host = Some(candidates[0].to_owned());
        }
        _ => {
            return Err(
                "hostRules cannot contain more than one host-matching field \
                 - use \"matchHost\" only."
                    .to_owned(),
            );
        }
    }

    Ok(rule)
}

// ---------------------------------------------------------------------------
/// @parity lib/util/host-rules.ts full
// Public API
// ---------------------------------------------------------------------------
/// Add a host rule to the global registry.
///
/// Mirrors `add()` from `lib/util/host-rules.ts`.
pub fn add(rule: HostRule) -> Result<(), String> {
    add_with_legacy(rule, LegacyHostRule::default())
}

/// Add a host rule with optional legacy field migration.
#[allow(clippy::needless_pass_by_value)]
pub fn add_with_legacy(rule: HostRule, legacy: LegacyHostRule) -> Result<(), String> {
    let mut rule = migrate_rule(rule, &legacy)?;

    if let Some(ref mh) = rule.match_host.clone() {
        let massaged = massage_host_url(mh);
        rule.match_host = Some(massaged.clone());
        let resolved = Url::parse(&massaged)
            .ok()
            .and_then(|u| u.host_str().map(str::to_owned))
            .unwrap_or_else(|| massaged.clone());
        rule.resolved_host = Some(resolved);
    }

    HOST_RULES.with(|rules| rules.borrow_mut().push(rule));
    Ok(())
}

/// Rank a rule for sorting — higher rank wins.
///
/// Mirrors `hostRuleRank` from `lib/util/host-rules.ts`.
fn host_rule_rank(rule: &HostRule) -> u8 {
    match (
        rule.host_type.is_some() || rule.read_only.is_some(),
        rule.match_host.is_some(),
        rule.host_type.is_some(),
    ) {
        (true, true, _) => 3,
        (_, true, _) => 2,
        (_, _, true) => 1,
        _ => 0,
    }
}

/// Find the best-matching combined rule for the given search criteria.
///
/// Mirrors `find()` from `lib/util/host-rules.ts`.
pub fn find(search: &HostRuleSearch) -> CombinedHostRule {
    if search.host_type.is_none() && search.url.is_none() {
        tracing::warn!("Invalid hostRules search");
        return CombinedHostRule::default();
    }

    let matched = HOST_RULES.with(|rules| {
        let rules = rules.borrow();

        // Sort primarily by matchHost length (shorter first), then by rank (lower first)
        let mut sorted: Vec<&HostRule> = rules.iter().collect();
        sorted.sort_by(|a, b| {
            let len_a = a.match_host.as_deref().map_or(0, str::len);
            let len_b = b.match_host.as_deref().map_or(0, str::len);
            len_a
                .cmp(&len_b)
                .then_with(|| host_rule_rank(a).cmp(&host_rule_rank(b)))
        });

        let mut matched: Vec<HostRule> = Vec::new();
        for rule in sorted {
            let mut host_type_match = true;
            let mut host_match = true;
            let mut read_only_match = true;

            if rule.host_type.is_some() {
                host_type_match = search.host_type.as_deref() == rule.host_type.as_deref();
            }

            if rule.match_host.is_some() && rule.resolved_host.is_some() {
                host_match = false;
                if let Some(ref url) = search.url
                    && let Some(ref mh) = rule.match_host
                {
                    host_match = matches_host(url, mh);
                }
            }

            if rule.read_only.is_some() {
                read_only_match = search.read_only == rule.read_only;
                if read_only_match {
                    host_type_match = true;
                }
            }

            if host_type_match && read_only_match && host_match {
                matched.push(rule.clone());
            }
        }
        matched
    });

    // Merge all matched rules (last rule wins per field)
    let mut combined = CombinedHostRule::default();
    for rule in matched {
        if let Some(v) = rule.auth_type {
            combined.auth_type = Some(v);
        }
        if let Some(v) = rule.token {
            combined.token = Some(v);
        }
        if let Some(v) = rule.username {
            combined.username = Some(v);
        }
        if let Some(v) = rule.password {
            combined.password = Some(v);
        }
        if let Some(v) = rule.insecure_registry {
            combined.insecure_registry = Some(v);
        }
        if let Some(v) = rule.timeout {
            combined.timeout = Some(v);
        }
        if let Some(v) = rule.abort_on_error {
            combined.abort_on_error = Some(v);
        }
        if let Some(v) = rule.abort_ignore_status_codes {
            combined.abort_ignore_status_codes = Some(v);
        }
        if let Some(v) = rule.enabled {
            combined.enabled = Some(v);
        }
        if let Some(v) = rule.enable_http2 {
            combined.enable_http2 = Some(v);
        }
        if let Some(v) = rule.concurrent_request_limit {
            combined.concurrent_request_limit = Some(v);
        }
        if let Some(v) = rule.max_requests_per_second {
            combined.max_requests_per_second = Some(v);
        }
        if let Some(v) = rule.headers {
            combined.headers = Some(v);
        }
        if let Some(v) = rule.max_retry_after {
            combined.max_retry_after = Some(v);
        }
        if let Some(v) = rule.keep_alive {
            combined.keep_alive = Some(v);
        }
        if let Some(v) = rule.https_certificate_authority {
            combined.https_certificate_authority = Some(v);
        }
        if let Some(v) = rule.https_private_key {
            combined.https_private_key = Some(v);
        }
        if let Some(v) = rule.https_certificate {
            combined.https_certificate = Some(v);
        }
    }
    combined
}

/// List unique resolved hostnames for all rules matching `host_type`.
///
/// Mirrors `hosts()` from `lib/util/host-rules.ts`.
pub fn hosts(host_type: &str) -> Vec<String> {
    HOST_RULES.with(|rules| {
        rules
            .borrow()
            .iter()
            .filter(|r| r.host_type.as_deref() == Some(host_type))
            .filter_map(|r| r.resolved_host.clone())
            .collect()
    })
}

/// Return the best-matching `hostType` string for the given URL.
///
/// Mirrors `hostType()` from `lib/util/host-rules.ts`.
pub fn host_type_for_url(url: &str) -> Option<String> {
    HOST_RULES.with(|rules| {
        let rules = rules.borrow();
        let mut candidates: Vec<&HostRule> = rules
            .iter()
            .filter(|r| {
                r.match_host
                    .as_deref()
                    .is_some_and(|mh| matches_host(url, mh))
            })
            .collect();
        // Sort by matchHost length ascending; we want the *last* (longest) via `.pop()`
        candidates.sort_by_key(|r| r.match_host.as_deref().map_or(0, str::len));
        candidates.pop().and_then(|r| r.host_type.clone())
    })
}

/// Find all rules whose `host_type` field equals the given value.
///
/// Mirrors `findAll()` from `lib/util/host-rules.ts`.
pub fn find_all(host_type: &str) -> Vec<HostRule> {
    HOST_RULES.with(|rules| {
        rules
            .borrow()
            .iter()
            .filter(|r| r.host_type.as_deref() == Some(host_type))
            .cloned()
            .collect()
    })
}

/// Return a snapshot of all stored rules.
///
/// Mirrors `getAll()` from `lib/util/host-rules.ts`.
pub fn get_all() -> Vec<HostRule> {
    HOST_RULES.with(|rules| rules.borrow().clone())
}

/// Clear all host rules.
///
/// Mirrors `clear()` from `lib/util/host-rules.ts`.
pub fn clear() {
    HOST_RULES.with(|rules| rules.borrow_mut().clear());
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() {
        clear();
    }

    // ── add() ────────────────────────────────────────────────────────────────

    // Ported: "throws if both domainName and hostName" — lib/util/host-rules.spec.ts line 18
    #[test]
    fn add_throws_if_both_domain_name_and_host_name() {
        setup();
        let result = add_with_legacy(
            HostRule {
                host_type: Some("azure".to_owned()),
                ..Default::default()
            },
            LegacyHostRule {
                domain_name: Some("github.com".to_owned()),
                host_name: Some("api.github.com".to_owned()),
                ..Default::default()
            },
        );
        assert!(result.is_err());
    }

    // Ported: "throws if both domainName and baseUrl" — lib/util/host-rules.spec.ts line 28
    #[test]
    fn add_throws_if_both_domain_name_and_base_url() {
        setup();
        let result = add_with_legacy(
            HostRule {
                host_type: Some("azure".to_owned()),
                match_host: Some("https://api.github.com".to_owned()),
                ..Default::default()
            },
            LegacyHostRule {
                domain_name: Some("github.com".to_owned()),
                ..Default::default()
            },
        );
        assert!(result.is_err());
    }

    // Ported: "throws if both hostName and baseUrl" — lib/util/host-rules.spec.ts line 38
    #[test]
    fn add_throws_if_both_host_name_and_base_url() {
        setup();
        let result = add_with_legacy(
            HostRule {
                host_type: Some("azure".to_owned()),
                match_host: Some("https://api.github.com".to_owned()),
                ..Default::default()
            },
            LegacyHostRule {
                host_name: Some("api.github.com".to_owned()),
                ..Default::default()
            },
        );
        assert!(result.is_err());
    }

    // Ported: "supports baseUrl-only" — lib/util/host-rules.spec.ts line 48
    #[test]
    fn add_supports_base_url_only() {
        setup();
        add(HostRule {
            match_host: Some("https://some.endpoint".to_owned()),
            username: Some("user1".to_owned()),
            password: Some("pass1".to_owned()),
            ..Default::default()
        })
        .unwrap();

        let search = HostRuleSearch {
            url: Some("https://some.endpoint/v3/".to_owned()),
            ..Default::default()
        };
        let result = find(&search);
        assert_eq!(result.username.as_deref(), Some("user1"));
        assert_eq!(result.password.as_deref(), Some("pass1"));

        let result2 = find(&HostRuleSearch {
            url: Some("https://some.endpoint/".to_owned()),
            ..Default::default()
        });
        assert_eq!(result2.username.as_deref(), Some("user1"));

        let result3 = find(&HostRuleSearch {
            url: Some("https://some.endpoint".to_owned()),
            ..Default::default()
        });
        assert_eq!(result3.username.as_deref(), Some("user1"));

        // Port 443 is default for https — normalized away
        let result4 = find(&HostRuleSearch {
            url: Some("https://some.endpoint:443".to_owned()),
            ..Default::default()
        });
        assert_eq!(result4.username.as_deref(), Some("user1"));
    }

    // Ported: "does not match subpart of hostname" — lib/util/host-rules.spec.ts line 72
    #[test]
    fn add_does_not_match_subpart_of_hostname() {
        setup();
        add(HostRule {
            match_host: Some("https://some.endpoint".to_owned()),
            username: Some("user1".to_owned()),
            password: Some("pass1".to_owned()),
            ..Default::default()
        })
        .unwrap();

        assert_eq!(
            find(&HostRuleSearch {
                url: Some("https://some.endpoint.example.com".to_owned()),
                ..Default::default()
            }),
            CombinedHostRule::default()
        );
        assert_eq!(
            find(&HostRuleSearch {
                url: Some("https://some.endpoint:blub@example.com".to_owned()),
                ..Default::default()
            }),
            CombinedHostRule::default()
        );
    }

    // Ported: "massages host url" — lib/util/host-rules.spec.ts line 84
    #[test]
    fn add_massages_host_url() {
        setup();
        add(HostRule {
            match_host: Some("some.domain.com:8080".to_owned()),
            username: Some("user1".to_owned()),
            password: Some("pass1".to_owned()),
            ..Default::default()
        })
        .unwrap();
        add(HostRule {
            match_host: Some("domain.com/".to_owned()),
            username: Some("user2".to_owned()),
            password: Some("pass2".to_owned()),
            ..Default::default()
        })
        .unwrap();

        let r1 = find(&HostRuleSearch {
            url: Some("https://some.domain.com:8080".to_owned()),
            ..Default::default()
        });
        assert_eq!(r1.username.as_deref(), Some("user1"));

        let r2 = find(&HostRuleSearch {
            url: Some("https://domain.com/".to_owned()),
            ..Default::default()
        });
        assert_eq!(r2.username.as_deref(), Some("user2"));
    }

    // ── find() ───────────────────────────────────────────────────────────────

    // Ported: "warns and returns empty for bad search" — lib/util/host-rules.spec.ts line 111
    #[test]
    fn find_warns_and_returns_empty_for_bad_search() {
        setup();
        let result = find(&HostRuleSearch::default());
        assert_eq!(result, CombinedHostRule::default());
    }

    // Ported: "needs exact host matches" — lib/util/host-rules.spec.ts line 115
    #[test]
    fn find_needs_exact_host_matches() {
        setup();
        add_with_legacy(
            HostRule {
                host_type: Some("nuget".to_owned()),
                username: Some("root".to_owned()),
                password: Some("p4$$w0rd".to_owned()),
                ..Default::default()
            },
            LegacyHostRule {
                host_name: Some("nuget.org".to_owned()),
                ..Default::default()
            },
        )
        .unwrap();

        // host-only search — no URL, so matchHost rule doesn't fire
        let r1 = find(&HostRuleSearch {
            host_type: Some("nuget".to_owned()),
            ..Default::default()
        });
        assert_eq!(r1, CombinedHostRule::default());

        // URL that matches nuget.org
        let r2 = find(&HostRuleSearch {
            host_type: Some("nuget".to_owned()),
            url: Some("https://nuget.org".to_owned()),
            ..Default::default()
        });
        assert_ne!(r2, CombinedHostRule::default());

        // not.nuget.org is a subdomain → matches domain rule
        let r3 = find(&HostRuleSearch {
            host_type: Some("nuget".to_owned()),
            url: Some("https://not.nuget.org".to_owned()),
            ..Default::default()
        });
        assert_ne!(r3, CombinedHostRule::default());

        // not-nuget.org is a different domain → no match
        let r4 = find(&HostRuleSearch {
            host_type: Some("nuget".to_owned()),
            url: Some("https://not-nuget.org".to_owned()),
            ..Default::default()
        });
        assert_eq!(r4, CombinedHostRule::default());
    }

    // Ported: "matches on empty rules" — lib/util/host-rules.spec.ts line 135
    #[test]
    fn find_matches_on_empty_rules() {
        setup();
        add(HostRule {
            enabled: Some(true),
            ..Default::default()
        })
        .unwrap();
        let r = find(&HostRuleSearch {
            host_type: Some("nuget".to_owned()),
            url: Some("https://api.github.com".to_owned()),
            ..Default::default()
        });
        assert_eq!(r.enabled, Some(true));
    }

    // Ported: "matches on hostType" — lib/util/host-rules.spec.ts line 144
    #[test]
    fn find_matches_on_host_type() {
        setup();
        add(HostRule {
            host_type: Some("nuget".to_owned()),
            token: Some("abc".to_owned()),
            ..Default::default()
        })
        .unwrap();
        let r = find(&HostRuleSearch {
            host_type: Some("nuget".to_owned()),
            url: Some("https://nuget.local/api".to_owned()),
            ..Default::default()
        });
        assert_eq!(r.token.as_deref(), Some("abc"));
    }

    // Ported: "matches on domainName" — lib/util/host-rules.spec.ts line 154
    #[test]
    fn find_matches_on_domain_name() {
        setup();
        add_with_legacy(
            HostRule {
                token: Some("def".to_owned()),
                ..Default::default()
            },
            LegacyHostRule {
                domain_name: Some("github.com".to_owned()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(
            find(&HostRuleSearch {
                host_type: Some("nuget".to_owned()),
                url: Some("https://api.github.com".to_owned()),
                ..Default::default()
            })
            .token
            .as_deref(),
            Some("def")
        );
        assert_eq!(
            find(&HostRuleSearch {
                host_type: Some("nuget".to_owned()),
                url: Some("https://github.com".to_owned()),
                ..Default::default()
            })
            .token
            .as_deref(),
            Some("def")
        );
        assert_eq!(
            find(&HostRuleSearch {
                host_type: Some("nuget".to_owned()),
                url: Some("https://apigithub.com".to_owned()),
                ..Default::default()
            })
            .token,
            None
        );
    }

    // Ported: "matches on specific path" — lib/util/host-rules.spec.ts line 172
    #[test]
    fn find_matches_on_specific_path() {
        setup();
        add(HostRule {
            host_type: Some("github".to_owned()),
            match_host: Some("https://api.github.com".to_owned()),
            token: Some("abc".to_owned()),
            ..Default::default()
        })
        .unwrap();
        add(HostRule {
            host_type: Some("github".to_owned()),
            match_host: Some("https://api.github.com/repos/org-b/".to_owned()),
            token: Some("def".to_owned()),
            ..Default::default()
        })
        .unwrap();
        add(HostRule {
            host_type: Some("github".to_owned()),
            match_host: Some("https://api.github.com".to_owned()),
            token: Some("abc".to_owned()),
            ..Default::default()
        })
        .unwrap();

        let r = find(&HostRuleSearch {
            host_type: Some("github".to_owned()),
            url: Some("https://api.github.com/repos/org-b/someRepo/tags?per_page=100".to_owned()),
            ..Default::default()
        });
        assert_eq!(r.token.as_deref(), Some("def"));
    }

    // Ported: "matches for several hostTypes when no hostType rule is configured" — lib/util/host-rules.spec.ts line 199
    #[test]
    fn find_matches_for_several_host_types() {
        setup();
        add(HostRule {
            match_host: Some("https://api.github.com".to_owned()),
            token: Some("abc".to_owned()),
            ..Default::default()
        })
        .unwrap();
        // Rule without hostType should match regardless of searched hostType
        let r = find(&HostRuleSearch {
            host_type: Some("github".to_owned()),
            url: Some("https://api.github.com".to_owned()),
            ..Default::default()
        });
        assert_eq!(r.token.as_deref(), Some("abc"));
    }

    // Ported: "matches if hostType is configured and host rule is filtered with datasource" — lib/util/host-rules.spec.ts line 218
    #[test]
    fn find_matches_if_host_type_filtered_with_datasource() {
        setup();
        add(HostRule {
            host_type: Some("github".to_owned()),
            match_host: Some("https://api.github.com".to_owned()),
            token: Some("abc".to_owned()),
            ..Default::default()
        })
        .unwrap();
        add(HostRule {
            host_type: Some("github-tags".to_owned()),
            match_host: Some("https://api.github.com/repos/org-b/".to_owned()),
            token: Some("def".to_owned()),
            ..Default::default()
        })
        .unwrap();

        let r = find(&HostRuleSearch {
            host_type: Some("github-tags".to_owned()),
            url: Some("https://api.github.com/repos/org-b/someRepo/tags?per_page=100".to_owned()),
            ..Default::default()
        });
        assert_eq!(r.token.as_deref(), Some("def"));
    }

    // Ported: "matches on hostName" — lib/util/host-rules.spec.ts line 237
    #[test]
    fn find_matches_on_host_name() {
        setup();
        add_with_legacy(
            HostRule {
                token: Some("abc".to_owned()),
                ..Default::default()
            },
            LegacyHostRule {
                host_name: Some("api.github.com".to_owned()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(
            find(&HostRuleSearch {
                url: Some("https://api.github.com".to_owned()),
                ..Default::default()
            })
            .token
            .as_deref(),
            Some("abc")
        );
    }

    // Ported: "matches on matchHost with protocol" — lib/util/host-rules.spec.ts line 247
    #[test]
    fn find_matches_on_match_host_with_protocol() {
        setup();
        add(HostRule {
            match_host: Some("https://domain.com".to_owned()),
            token: Some("def".to_owned()),
            ..Default::default()
        })
        .unwrap();

        // Subdomain should NOT match when matchHost uses https:// prefix
        assert_eq!(
            find(&HostRuleSearch {
                url: Some("https://api.domain.com".to_owned()),
                ..Default::default()
            })
            .token,
            None
        );
        assert_eq!(
            find(&HostRuleSearch {
                url: Some("https://domain.com".to_owned()),
                ..Default::default()
            })
            .token
            .as_deref(),
            Some("def")
        );
        assert_eq!(
            find(&HostRuleSearch {
                url: Some("https://domain.com/renovatebot".to_owned()),
                ..Default::default()
            })
            .token
            .as_deref(),
            Some("def")
        );
        // http:// should NOT match https://
        assert_eq!(
            find(&HostRuleSearch {
                url: Some("http://domain.com/some/path".to_owned()),
                ..Default::default()
            })
            .token,
            None
        );
    }

    // Ported: "matches on matchHost without protocol" — lib/util/host-rules.spec.ts line 262
    #[test]
    fn find_matches_on_match_host_without_protocol() {
        setup();
        add(HostRule {
            match_host: Some("domain.com".to_owned()),
            token: Some("def".to_owned()),
            ..Default::default()
        })
        .unwrap();

        // Without protocol, subdomains DO match
        assert_eq!(
            find(&HostRuleSearch {
                url: Some("https://api.domain.com".to_owned()),
                ..Default::default()
            })
            .token
            .as_deref(),
            Some("def")
        );
        assert_eq!(
            find(&HostRuleSearch {
                url: Some("https://domain.com".to_owned()),
                ..Default::default()
            })
            .token
            .as_deref(),
            Some("def")
        );
        // Malformed URL (no scheme)
        assert_eq!(
            find(&HostRuleSearch {
                url: Some("httpsdomain.com".to_owned()),
                ..Default::default()
            })
            .token,
            None
        );
    }

    // Ported: "matches on matchHost with dot prefix" — lib/util/host-rules.spec.ts line 272
    #[test]
    fn find_matches_on_match_host_with_dot_prefix() {
        setup();
        add(HostRule {
            match_host: Some(".domain.com".to_owned()),
            token: Some("def".to_owned()),
            ..Default::default()
        })
        .unwrap();

        assert_eq!(
            find(&HostRuleSearch {
                url: Some("https://subdomain.domain.com:9118".to_owned()),
                ..Default::default()
            })
            .token
            .as_deref(),
            Some("def")
        );
        // Exact domain without subdomain should NOT match ".domain.com"
        assert_eq!(
            find(&HostRuleSearch {
                url: Some("https://domain.com".to_owned()),
                ..Default::default()
            })
            .token,
            None
        );
        // Malformed URL
        assert_eq!(
            find(&HostRuleSearch {
                url: Some("httpsdomain.com".to_owned()),
                ..Default::default()
            })
            .token,
            None
        );
    }

    // Ported: "matches on matchHost with port" — lib/util/host-rules.spec.ts line 282
    #[test]
    fn find_matches_on_match_host_with_port() {
        setup();
        add(HostRule {
            match_host: Some("domain.com:9118".to_owned()),
            token: Some("def".to_owned()),
            ..Default::default()
        })
        .unwrap();

        assert_eq!(
            find(&HostRuleSearch {
                url: Some("https://domain.com:9118".to_owned()),
                ..Default::default()
            })
            .token
            .as_deref(),
            Some("def")
        );
        assert_eq!(
            find(&HostRuleSearch {
                url: Some("https://domain.com".to_owned()),
                ..Default::default()
            })
            .token,
            None
        );
    }

    // Ported: "matches on hostType and endpoint" — lib/util/host-rules.spec.ts line 292
    #[test]
    fn find_matches_on_host_type_and_endpoint() {
        setup();
        add(HostRule {
            host_type: Some("nuget".to_owned()),
            match_host: Some("https://nuget.local/api".to_owned()),
            token: Some("abc".to_owned()),
            ..Default::default()
        })
        .unwrap();
        let r = find(&HostRuleSearch {
            host_type: Some("nuget".to_owned()),
            url: Some("https://nuget.local/api".to_owned()),
            ..Default::default()
        });
        assert_eq!(r.token.as_deref(), Some("abc"));
    }

    // Ported: "matches on endpoint subresource" — lib/util/host-rules.spec.ts line 304
    #[test]
    fn find_matches_on_endpoint_subresource() {
        setup();
        add(HostRule {
            host_type: Some("nuget".to_owned()),
            match_host: Some("https://nuget.local/api".to_owned()),
            token: Some("abc".to_owned()),
            ..Default::default()
        })
        .unwrap();
        let r = find(&HostRuleSearch {
            host_type: Some("nuget".to_owned()),
            url: Some("https://nuget.local/api/sub-resource".to_owned()),
            ..Default::default()
        });
        assert_eq!(r.token.as_deref(), Some("abc"));
    }

    // Ported: "matches shortest matchHost first" — lib/util/host-rules.spec.ts line 318
    #[test]
    fn find_matches_shortest_match_host_first() {
        setup();
        add(HostRule {
            match_host: Some("https://nuget.local/api".to_owned()),
            token: Some("longest".to_owned()),
            ..Default::default()
        })
        .unwrap();
        add(HostRule {
            match_host: Some("https://nuget.local/".to_owned()),
            token: Some("shortest".to_owned()),
            ..Default::default()
        })
        .unwrap();
        let r = find(&HostRuleSearch {
            url: Some("https://nuget.local/api/sub-resource".to_owned()),
            ..Default::default()
        });
        // Longer match wins (last write wins after sorting by length ascending)
        assert_eq!(r.token.as_deref(), Some("longest"));
    }

    // Ported: "matches readOnly requests" — lib/util/host-rules.spec.ts line 334
    #[test]
    fn find_matches_read_only_requests() {
        setup();
        add(HostRule {
            match_host: Some("https://api.github.com/repos/".to_owned()),
            token: Some("aaa".to_owned()),
            host_type: Some("github".to_owned()),
            ..Default::default()
        })
        .unwrap();
        add(HostRule {
            match_host: Some("https://api.github.com".to_owned()),
            token: Some("bbb".to_owned()),
            read_only: Some(true),
            ..Default::default()
        })
        .unwrap();
        let r = find(&HostRuleSearch {
            url: Some("https://api.github.com/repos/foo/bar/tags".to_owned()),
            read_only: Some(true),
            ..Default::default()
        });
        assert_eq!(r.token.as_deref(), Some("bbb"));
    }

    // ── hosts() ──────────────────────────────────────────────────────────────

    // Ported: "returns hosts" — lib/util/host-rules.spec.ts line 355
    #[test]
    fn hosts_returns_hosts() {
        setup();
        add(HostRule {
            host_type: Some("nuget".to_owned()),
            token: Some("aaaaaa".to_owned()),
            ..Default::default()
        })
        .unwrap();
        add(HostRule {
            host_type: Some("nuget".to_owned()),
            match_host: Some("https://nuget.local/api".to_owned()),
            token: Some("abc".to_owned()),
            ..Default::default()
        })
        .unwrap();
        add_with_legacy(
            HostRule {
                host_type: Some("nuget".to_owned()),
                token: Some("def".to_owned()),
                ..Default::default()
            },
            LegacyHostRule {
                host_name: Some("my.local.registry".to_owned()),
                ..Default::default()
            },
        )
        .unwrap();
        add(HostRule {
            host_type: Some("nuget".to_owned()),
            match_host: Some("another.local.registry".to_owned()),
            token: Some("xyz".to_owned()),
            ..Default::default()
        })
        .unwrap();
        add(HostRule {
            host_type: Some("nuget".to_owned()),
            match_host: Some("https://yet.another.local.registry".to_owned()),
            token: Some("123".to_owned()),
            ..Default::default()
        })
        .unwrap();

        let result = hosts("nuget");
        assert_eq!(
            result,
            vec![
                "nuget.local",
                "my.local.registry",
                "another.local.registry",
                "yet.another.local.registry",
            ]
        );
    }

    // ── findAll() ────────────────────────────────────────────────────────────

    // Ported: "warns and returns empty for bad search" — lib/util/host-rules.spec.ts line 393
    #[test]
    fn find_all_returns_empty_for_unknown_host_type() {
        setup();
        let result = find_all("nonexistent");
        assert!(result.is_empty());
    }

    // Ported: "needs exact host matches" — lib/util/host-rules.spec.ts line 397
    #[test]
    fn find_all_needs_exact_host_matches() {
        setup();
        add_with_legacy(
            HostRule {
                host_type: Some("nuget".to_owned()),
                username: Some("root".to_owned()),
                password: Some("p4$$w0rd".to_owned()),
                ..Default::default()
            },
            LegacyHostRule {
                host_name: Some("nuget.org".to_owned()),
                ..Default::default()
            },
        )
        .unwrap();

        let results = find_all("nuget");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].host_type.as_deref(), Some("nuget"));
        assert_eq!(results[0].match_host.as_deref(), Some("nuget.org"));
        assert_eq!(results[0].resolved_host.as_deref(), Some("nuget.org"));
        assert_eq!(results[0].username.as_deref(), Some("root"));
        assert_eq!(results[0].password.as_deref(), Some("p4$$w0rd"));
    }

    // ── getAll() ─────────────────────────────────────────────────────────────

    // Ported: "returns all host rules" — lib/util/host-rules.spec.ts line 418
    #[test]
    fn get_all_returns_all_rules() {
        setup();
        add(HostRule {
            host_type: Some("nuget".to_owned()),
            match_host: Some("nuget.org".to_owned()),
            username: Some("root".to_owned()),
            password: Some("p4$$w0rd".to_owned()),
            ..Default::default()
        })
        .unwrap();
        add(HostRule {
            host_type: Some("github".to_owned()),
            match_host: Some("github.com".to_owned()),
            token: Some("token".to_owned()),
            ..Default::default()
        })
        .unwrap();

        let all = get_all();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].host_type.as_deref(), Some("nuget"));
        assert_eq!(all[1].host_type.as_deref(), Some("github"));
    }

    // ── hostType() ───────────────────────────────────────────────────────────

    // Ported: "return hostType" — lib/util/host-rules.spec.ts line 437
    #[test]
    fn host_type_for_url_returns_host_type() {
        setup();
        add(HostRule {
            host_type: Some("github".to_owned()),
            token: Some("aaaaaa".to_owned()),
            ..Default::default()
        })
        .unwrap();
        add(HostRule {
            host_type: Some("github".to_owned()),
            match_host: Some("github.example.com".to_owned()),
            token: Some("abc".to_owned()),
            ..Default::default()
        })
        .unwrap();
        add(HostRule {
            host_type: Some("github-changelog".to_owned()),
            match_host: Some("https://github.example.com/chalk/chalk".to_owned()),
            token: Some("def".to_owned()),
            ..Default::default()
        })
        .unwrap();

        assert_eq!(
            host_type_for_url("https://github.example.com/chalk/chalk"),
            Some("github-changelog".to_owned())
        );
    }

    // Ported: "returns null" — lib/util/host-rules.spec.ts line 459
    #[test]
    fn host_type_for_url_returns_none_for_no_match() {
        setup();
        add(HostRule {
            host_type: Some("github".to_owned()),
            token: Some("aaaaaa".to_owned()),
            ..Default::default()
        })
        .unwrap();
        add(HostRule {
            host_type: Some("github".to_owned()),
            match_host: Some("github.example.com".to_owned()),
            token: Some("abc".to_owned()),
            ..Default::default()
        })
        .unwrap();
        add(HostRule {
            host_type: Some("github-changelog".to_owned()),
            match_host: Some("https://github.example.com/chalk/chalk".to_owned()),
            token: Some("def".to_owned()),
            ..Default::default()
        })
        .unwrap();

        assert_eq!(
            host_type_for_url("https://github.example.com/chalk/chalk"),
            Some("github-changelog".to_owned())
        );
        assert_eq!(
            host_type_for_url("https://gitlab.example.com/chalk/chalk"),
            None
        );
    }

    #[test]
    fn massage_host_url_adds_https() {
        assert_eq!(
            massage_host_url("github.com/owner"),
            "https://github.com/owner"
        );
        assert_eq!(massage_host_url("https://github.com"), "https://github.com");
    }

    #[test]
    fn matches_host_basic() {
        assert!(matches_host("https://github.com/owner/repo", "github.com"));
        assert!(!matches_host("https://gitlab.com/owner/repo", "github.com"));
    }

    #[test]
    fn migrate_rule_basic() {
        let rule = HostRule::default();
        let legacy = LegacyHostRule {
            host_name: Some("github.com".to_owned()),
            ..Default::default()
        };
        let result = migrate_rule(rule, &legacy).unwrap();
        assert_eq!(result.match_host, Some("github.com".to_owned()));
    }
}

//! @parity lib/versioning-list.generated.ts partial
//! @parity lib/renovate.ts partial
//! @parity lib/manager-list.generated.ts partial
//! @parity lib/manager-default-configs.generated.ts partial
//! @parity lib/logger/with-sanitizer.ts partial
//! @parity lib/logger/utils.ts partial
//! @parity lib/logger/types.ts partial
//! @parity lib/logger/renovate-logger.ts partial
//! @parity lib/logger/remap.ts partial
//! @parity lib/logger/problem-stream.ts partial
//! @parity lib/logger/pretty-stdout.ts partial
//! @parity lib/logger/once.ts partial
//! @parity lib/logger/index.ts partial
//! @parity lib/logger/err-serializer.ts partial
//! @parity lib/logger/config-serializer.ts partial
//! @parity lib/logger/cmd-serializer.ts partial
//! @parity lib/logger/bunyan.ts partial
//! @parity lib/instrumentation/with-instrumenting.ts partial
//! @parity lib/instrumentation/utils.ts partial
//! @parity lib/instrumentation/types.ts partial
//! @parity lib/instrumentation/reporting.ts partial
//! @parity lib/instrumentation/index.ts partial
//! @parity lib/instrumentation/detectors.ts partial
//! @parity lib/global-config-option-defaults.generated.ts partial
//! @parity lib/expose.ts partial
//! @parity lib/datasource-list.generated.ts partial
//! @parity lib/data-files.generated.ts partial
//! @parity lib/config-validator.ts partial
//! Core domain types for the Rust reimplementation of Renovate.

// hygiene allow for pre-existing -D lints in debt (unreachable, dead, etc)
#![allow(unreachable_patterns, dead_code, unused_variables, unused_imports, unused_qualifications, elided_lifetimes_in_paths, single_use_lifetimes, trivial_casts, let_underscore_drop)]

pub mod artifacts;
pub mod branch;
pub mod cache;
pub mod config;
pub mod datasources;
pub mod exec;
pub mod extractors;
pub mod fs;
pub mod git;
pub mod http;
pub mod json_writer;
pub mod limits;
pub mod managers;
pub mod merge_confidence;
pub mod monorepos;
pub mod onboarding;
pub mod package_rule;
pub mod platform;
pub mod platform_constants;
pub mod proxy;
pub mod replacements;
pub mod repo_config;
pub mod schedule;
pub mod string_match;
pub mod timestamp;
pub mod util;
pub mod versioning;
pub mod vulnerability;
pub mod workers;

/// Library version string, sourced from the workspace package version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use serde::de::{self, DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor};

    struct KeyOrderChecker<'a> {
        file: &'a str,
        path: String,
        depth: usize,
        first_keys: &'a [&'a str],
    }

    impl<'de> DeserializeSeed<'de> for KeyOrderChecker<'_> {
        type Value = ();

        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_any(self)
        }
    }

    impl<'de> Visitor<'de> for KeyOrderChecker<'_> {
        type Value = ();

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("any JSON value")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut keys = Vec::new();

            while let Some(key) = map.next_key::<String>()? {
                if self.depth == 0 && key == "$schema" {
                    map.next_value::<IgnoredAny>()?;
                    continue;
                }

                let child_path = format!("{}.{}", self.path, key);
                keys.push(key);
                map.next_value_seed(KeyOrderChecker {
                    file: self.file,
                    path: child_path,
                    depth: self.depth + 1,
                    first_keys: &[],
                })?;
            }

            let mut sorted_keys = keys.as_slice();
            if self.depth == 0 && !self.first_keys.is_empty() {
                let actual_first = &keys[..self.first_keys.len().min(keys.len())];
                if actual_first != self.first_keys {
                    return Err(de::Error::custom(format!(
                        "{} should start with [{}]",
                        self.file,
                        self.first_keys.join(", ")
                    )));
                }
                sorted_keys = &keys[self.first_keys.len().min(keys.len())..];
            }

            let mut expected = sorted_keys.to_vec();
            expected.sort();
            if sorted_keys != expected {
                return Err(de::Error::custom(format!(
                    "{} keys should be sorted alphabetically",
                    self.path
                )));
            }

            Ok(())
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            while seq.next_element::<IgnoredAny>()?.is_some() {}
            Ok(())
        }

        fn visit_bool<E>(self, _v: bool) -> Result<Self::Value, E> {
            Ok(())
        }

        fn visit_i64<E>(self, _v: i64) -> Result<Self::Value, E> {
            Ok(())
        }

        fn visit_u64<E>(self, _v: u64) -> Result<Self::Value, E> {
            Ok(())
        }

        fn visit_f64<E>(self, _v: f64) -> Result<Self::Value, E> {
            Ok(())
        }

        fn visit_str<E>(self, _v: &str) -> Result<Self::Value, E> {
            Ok(())
        }

        fn visit_none<E>(self) -> Result<Self::Value, E> {
            Ok(())
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(())
        }
    }

    fn assert_json_keys_sorted(file: &str, content: &str, first_keys: &[&str]) {
        let mut deserializer = serde_json::Deserializer::from_str(content);
        KeyOrderChecker {
            file,
            path: file.to_owned(),
            depth: 0,
            first_keys,
        }
        .deserialize(&mut deserializer)
        .unwrap();
    }

    // Ported: "keys are sorted alphabetically" — lib/data/index.spec.ts line 55
    #[test]
    fn embedded_data_keys_are_sorted_alphabetically() {
        assert_json_keys_sorted("monorepo.json", include_str!("../data/monorepo.json"), &[]);
        assert_json_keys_sorted(
            "replacements.json",
            include_str!("../data/replacements.json"),
            &["all"],
        );
    }
}

//! Core worker types.
//!
//! Mirrors `lib/workers/types.ts`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::repository::update::pr::types::ChangeLogResult;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RenovateConfig {
    pub branch_prefix: Option<String>,
    pub npmrc: Option<String>,
    pub inherit_config: Option<bool>,
    pub inherit_config_repo_name: Option<String>,
    pub inherit_config_file_name: Option<String>,
    pub inherit_config_strict: Option<bool>,
    pub top_level_org: Option<String>,
    pub parent_org: Option<String>,
    pub secrets: Option<serde_json::Value>,
    pub variables: Option<serde_json::Value>,
    pub ignore_presets: Option<Vec<String>>,
    pub config_file_names: Option<Vec<String>>,
    pub additional_branch_prefix: Option<String>,
    pub branch_name: Option<String>,
    pub commit_message: Option<String>,
    pub commit_message_action: Option<String>,
    pub commit_message_topic: Option<String>,
    pub commit_message_extra: Option<String>,
    pub commit_message_suffix: Option<String>,
    pub commit_message_prefix: Option<String>,
    pub semantic_commits: Option<String>,
    pub semantic_commit_type: Option<String>,
    pub semantic_commit_scope: Option<String>,
    pub separate_major_minor: Option<bool>,
    pub separate_multiple_major: Option<bool>,
    pub separate_minor_patch: Option<bool>,
    pub separate_multiple_minor: Option<bool>,
    pub pr_hourly_limit: Option<i64>,
    pub pr_concurrent_limit: Option<i64>,
    pub branch_concurrent_limit: Option<i64>,
    pub commit_hourly_limit: Option<i64>,
    pub pr_priority: Option<i32>,
    pub labels: Option<Vec<String>>,
    pub add_labels: Option<Vec<String>>,
    pub reviewers: Option<Vec<String>>,
    pub additional_reviewers: Option<Vec<String>>,
    pub package_rules: Option<Vec<serde_json::Value>>,
    pub enabled: Option<bool>,
    pub managers: Option<Vec<String>>,
    pub datasources: Option<Vec<String>>,
    pub update_types: Option<Vec<String>>,
    pub ignore_paths: Option<Vec<String>>,
    pub include_paths: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Upgrade {
    pub dep_name: Option<String>,
    pub dep_type: Option<String>,
    pub current_value: Option<String>,
    pub current_version: Option<String>,
    pub current_digest: Option<String>,
    pub new_value: Option<String>,
    pub new_version: Option<String>,
    pub new_digest: Option<String>,
    pub new_major: Option<u64>,
    pub new_minor: Option<u64>,
    pub new_patch: Option<u64>,
    pub new_name: Option<String>,
    pub datasource: Option<String>,
    pub package_name: Option<String>,
    pub manager: Option<String>,
    pub update_type: Option<String>,
    pub is_pin: Option<bool>,
    pub is_bump: Option<bool>,
    pub is_major: Option<bool>,
    pub is_minor: Option<bool>,
    pub is_patch: Option<bool>,
    pub is_digest: Option<bool>,
    pub is_lock_file_update: Option<bool>,
    pub is_range: Option<bool>,
    pub is_rollback: Option<bool>,
    pub is_replacement: Option<bool>,
    pub is_single_version: Option<bool>,
    pub is_breaking: Option<bool>,
    pub package_file: Option<String>,
    pub lock_file: Option<String>,
    pub registry_url: Option<String>,
    pub source_url: Option<String>,
    pub source_directory: Option<String>,
    pub homepage: Option<String>,
    pub changelog_url: Option<String>,
    pub dependency_url: Option<String>,
    pub dep_index: Option<usize>,
    pub pretty_new_major: Option<String>,
    pub pretty_new_version: Option<String>,
    pub display_from: Option<String>,
    pub display_to: Option<String>,
    pub release_timestamp: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BranchUpgrade {
    #[serde(flatten)]
    pub upgrade: Upgrade,
    #[serde(flatten)]
    pub config: RenovateConfig,
    pub branch_name: Option<String>,
    pub base_branch: Option<String>,
    pub commit_body: Option<String>,
    pub commit_message: Option<String>,
    pub pr_title: Option<String>,
    pub pr_body_notes: Option<Vec<String>>,
    pub pr_header: Option<String>,
    pub pr_footer: Option<String>,
    pub group_name: Option<String>,
    pub group_slug: Option<String>,
    pub is_group: Option<bool>,
    pub is_lock_file_maintenance: Option<bool>,
    pub is_remediation: Option<bool>,
    pub dep_name_linked: Option<String>,
    pub dep_name_sanitized: Option<String>,
    pub github_name: Option<String>,
    pub has_release_notes: Option<bool>,
    pub releases: Option<Vec<ReleaseWithNotes>>,
    pub log_json: Option<ChangeLogResult>,
    pub minimum_confidence: Option<String>,
    pub artifact_errors: Option<Vec<ArtifactError>>,
    pub updated_package_files: Option<Vec<FileChange>>,
    pub updated_artifacts: Option<Vec<FileChange>>,
    pub reuse_existing_branch: Option<bool>,
    pub constraints: Option<HashMap<String, String>>,
    pub references: Option<String>,
    pub source_repo: Option<String>,
    pub source_repo_org: Option<String>,
    pub source_repo_name: Option<String>,
    pub auto_replace_string_template: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReleaseWithNotes {
    pub version: Option<String>,
    pub release_timestamp: Option<String>,
    pub git_ref: Option<String>,
    pub body: Option<String>,
    pub url: Option<String>,
    pub name: Option<String>,
    pub compare_url: Option<String>,
    pub is_rollback: Option<bool>,
    pub changes: Option<Vec<ChangeLogChange>>,
    pub release_notes_url: Option<String>,
    pub release_notes_source_url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArtifactError {
    pub lock_file: Option<String>,
    pub stderr: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub contents: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChangeLogChange {
    pub date: Option<String>,
    pub message: Option<String>,
    pub sha: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationMessage {
    pub topic: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchResult {
    AlreadyExisted,
    Automerged,
    #[default]
    Done,
    Error,
    NeedsApproval,
    NeedsPrApproval,
    NotScheduled,
    NoWork,
    Pending,
    PrCreated,
    PrEdited,
    PrLimitReached,
    CommitPerRunLimitReached,
    CommitHourlyLimitReached,
    BranchLimitReached,
    Rebase,
    UpdateNotScheduled,
    MinimumGroupSizeNotMet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrBlockedBy {
    BranchAutomerge,
    NeedsApproval,
    AwaitingTests,
    RateLimited,
    Error,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BranchMetadata {
    pub branch_name: String,
    pub branch_sha: Option<String>,
    pub base_branch: Option<String>,
    pub base_branch_sha: Option<String>,
    pub automerge: Option<bool>,
    pub is_modified: Option<bool>,
    pub is_pristine: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BaseBranchMetadata {
    pub branch_name: String,
    pub sha: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BranchSummary {
    pub base_branches: Vec<BaseBranchMetadata>,
    pub branches: Vec<BranchMetadata>,
    pub cache_modified: Option<bool>,
    pub default_branch: Option<String>,
    pub inactive_branches: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkerExtractConfig {
    pub manager: String,
    pub file_list: Vec<String>,
    pub manager_file_patterns: Option<Vec<String>>,
    pub include_paths: Option<Vec<String>>,
    pub ignore_paths: Option<Vec<String>>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DepWarnings {
    pub warnings: Vec<String>,
    pub warning_files: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpgradeFingerprintConfig {
    pub auto_replace_string_template: Option<String>,
    pub current_digest: Option<String>,
    pub current_value: Option<String>,
    pub current_version: Option<String>,
    pub datasource: Option<String>,
    pub dep_name: Option<String>,
    pub lock_file: Option<String>,
    pub locked_version: Option<String>,
    pub manager: Option<String>,
    pub new_name: Option<String>,
    pub new_digest: Option<String>,
    pub new_value: Option<String>,
    pub new_version: Option<String>,
    pub package_file: Option<String>,
    pub replace_string: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renovate_config_default() {
        let cfg = RenovateConfig::default();
        assert!(cfg.branch_prefix.is_none());
        assert!(cfg.enabled.is_none());
    }

    #[test]
    fn upgrade_default() {
        let u = Upgrade::default();
        assert!(u.dep_name.is_none());
        assert!(u.current_value.is_none());
        assert!(u.new_value.is_none());
    }

    #[test]
    fn branch_result_default_is_done() {
        assert_eq!(BranchResult::default(), BranchResult::Done);
    }

    #[test]
    fn branch_result_variants() {
        let variants = [
            BranchResult::AlreadyExisted,
            BranchResult::Automerged,
            BranchResult::Done,
            BranchResult::Error,
            BranchResult::NeedsApproval,
            BranchResult::NeedsPrApproval,
            BranchResult::NotScheduled,
            BranchResult::NoWork,
            BranchResult::Pending,
            BranchResult::PrCreated,
            BranchResult::PrEdited,
            BranchResult::PrLimitReached,
            BranchResult::CommitPerRunLimitReached,
            BranchResult::CommitHourlyLimitReached,
            BranchResult::BranchLimitReached,
            BranchResult::Rebase,
            BranchResult::UpdateNotScheduled,
            BranchResult::MinimumGroupSizeNotMet,
        ];
        assert_eq!(variants.len(), 18);
    }

    #[test]
    fn pr_blocked_by_variants() {
        assert_ne!(PrBlockedBy::BranchAutomerge, PrBlockedBy::Error);
        assert_ne!(PrBlockedBy::NeedsApproval, PrBlockedBy::RateLimited);
    }

    #[test]
    fn branch_metadata_default() {
        let m = BranchMetadata::default();
        assert!(m.branch_name.is_empty());
        assert!(m.branch_sha.is_none());
    }

    #[test]
    fn branch_summary_default() {
        let s = BranchSummary::default();
        assert!(s.base_branches.is_empty());
        assert!(s.branches.is_empty());
        assert!(s.inactive_branches.is_empty());
    }

    #[test]
    fn upgrade_serialization_roundtrip() {
        let u = Upgrade {
            dep_name: Some("lodash".into()),
            current_value: Some("4.17.0".into()),
            new_value: Some("4.18.2".into()),
            datasource: Some("npm".into()),
            ..Default::default()
        };
        let json = serde_json::to_string(&u).unwrap();
        let back: Upgrade = serde_json::from_str(&json).unwrap();
        assert_eq!(back.dep_name, Some("lodash".into()));
        assert_eq!(back.current_value, Some("4.17.0".into()));
        assert_eq!(back.new_value, Some("4.18.2".into()));
    }

    #[test]
    fn branch_upgrade_flattened_fields() {
        let bu = BranchUpgrade {
            upgrade: Upgrade {
                dep_name: Some("react".into()),
                ..Default::default()
            },
            branch_name: Some("renovate/react-18.x".into()),
            ..Default::default()
        };
        assert_eq!(bu.upgrade.dep_name, Some("react".into()));
        assert_eq!(bu.branch_name, Some("renovate/react-18.x".into()));
    }

    #[test]
    fn artifact_error_default() {
        let e = ArtifactError::default();
        assert!(e.lock_file.is_none());
        assert!(e.stderr.is_none());
    }

    #[test]
    fn file_change_construct() {
        let fc = FileChange {
            path: "package.json".into(),
            contents: Some("{}".into()),
        };
        assert_eq!(fc.path, "package.json");
        assert_eq!(fc.contents, Some("{}".into()));
    }

    #[test]
    fn dep_warnings_default() {
        let dw = DepWarnings::default();
        assert!(dw.warnings.is_empty());
        assert!(dw.warning_files.is_empty());
    }

    #[test]
    fn upgrade_fingerprint_config_default() {
        let c = UpgradeFingerprintConfig::default();
        assert!(c.dep_name.is_none());
        assert!(c.current_value.is_none());
    }

    #[test]
    fn validation_message_construct() {
        let vm = ValidationMessage {
            topic: Some("dep".into()),
            message: Some("unsupported".into()),
        };
        assert_eq!(vm.topic, Some("dep".into()));
    }

    #[test]
    fn worker_extract_config_construct() {
        let c = WorkerExtractConfig {
            manager: "npm".into(),
            file_list: vec!["package.json".into()],
            enabled: Some(true),
            ..Default::default()
        };
        assert_eq!(c.manager, "npm");
        assert_eq!(c.file_list.len(), 1);
    }

    #[test]
    fn base_branch_metadata_construct() {
        let m = BaseBranchMetadata {
            branch_name: "main".into(),
            sha: "abc123".into(),
        };
        assert_eq!(m.branch_name, "main");
        assert_eq!(m.sha, "abc123");
    }
}

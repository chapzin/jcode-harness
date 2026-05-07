use anyhow::Result;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillImportStatus {
    Ok,
    Warn,
    Error,
}

impl SkillImportStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillImportSeverity {
    Info,
    Warning,
    Error,
}

impl SkillImportSeverity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillImportScope {
    Project,
    Global,
}

impl SkillImportScope {
    pub fn label(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Global => "global",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkillImportOptions {
    pub root: PathBuf,
    pub sources: Vec<PathBuf>,
    pub scope: SkillImportScope,
    pub apply: bool,
    pub force: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillImportTarget {
    pub scope: SkillImportScope,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillImportSourceSummary {
    pub origin: String,
    pub path: String,
    pub exists: bool,
    pub checked: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillImportFinding {
    pub severity: SkillImportSeverity,
    pub code: String,
    pub source_origin: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkillImportActionKind {
    Copy,
    Overwrite,
    SkipExisting,
    SkipInvalid,
    SkipSameTarget,
    Error,
}

impl SkillImportActionKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::Overwrite => "overwrite",
            Self::SkipExisting => "skip-existing",
            Self::SkipInvalid => "skip-invalid",
            Self::SkipSameTarget => "skip-same-target",
            Self::Error => "error",
        }
    }

    fn is_write_action(self) -> bool {
        matches!(self, Self::Copy | Self::Overwrite)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillImportAction {
    pub name: Option<String>,
    pub source_origin: String,
    pub source_path: String,
    pub target_path: String,
    pub action: SkillImportActionKind,
    pub applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub findings: Vec<SkillImportFinding>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillImportReport {
    pub status: SkillImportStatus,
    pub offline: bool,
    pub dry_run: bool,
    pub root: String,
    pub target: SkillImportTarget,
    pub force: bool,
    pub planned: usize,
    pub copied: usize,
    pub skipped: usize,
    pub errors: usize,
    pub warnings: usize,
    pub sources: Vec<SkillImportSourceSummary>,
    pub findings: Vec<SkillImportFinding>,
    pub actions: Vec<SkillImportAction>,
}

impl SkillImportReport {
    pub fn should_fail(&self) -> bool {
        self.errors > 0
    }
}

pub fn run_import(options: SkillImportOptions) -> Result<SkillImportReport> {
    let target_dir = target_dir(&options.root, options.scope)?;
    let explicit_sources = !options.sources.is_empty();
    let source_dirs = if explicit_sources {
        options.sources.clone()
    } else {
        default_source_dirs(&options.root)
    };

    let mut sources = Vec::new();
    let mut findings = Vec::new();
    let mut actions = Vec::new();

    for source_dir in source_dirs {
        collect_source_dir(
            &source_dir,
            &target_dir,
            explicit_sources,
            options.force,
            &mut sources,
            &mut findings,
            &mut actions,
        )?;
    }

    let mut copied = 0;
    if options.apply {
        for action in actions.iter_mut() {
            if !action.action.is_write_action() {
                continue;
            }
            if let Err(err) = copy_skill_dir(
                Path::new(&action.source_path),
                Path::new(&action.target_path),
                options.force,
            ) {
                let finding = finding(
                    SkillImportSeverity::Error,
                    "copy-failed",
                    &action.source_origin,
                    &action.source_path,
                    format!("Failed to copy skill into {}: {err}", action.target_path),
                );
                action.action = SkillImportActionKind::Error;
                action.reason = Some(finding.message.clone());
                action.findings.push(finding.clone());
                findings.push(finding);
            } else {
                action.applied = true;
                copied += 1;
            }
        }
    }

    let planned = actions
        .iter()
        .filter(|action| action.action.is_write_action())
        .count();
    let skipped = actions.len().saturating_sub(planned);
    let errors = findings
        .iter()
        .filter(|finding| finding.severity == SkillImportSeverity::Error)
        .count();
    let warnings = findings
        .iter()
        .filter(|finding| finding.severity == SkillImportSeverity::Warning)
        .count();
    let status = if errors > 0 {
        SkillImportStatus::Error
    } else if warnings > 0 {
        SkillImportStatus::Warn
    } else {
        SkillImportStatus::Ok
    };

    Ok(SkillImportReport {
        status,
        offline: true,
        dry_run: !options.apply,
        root: options.root.display().to_string(),
        target: SkillImportTarget {
            scope: options.scope,
            path: target_dir.display().to_string(),
        },
        force: options.force,
        planned,
        copied,
        skipped,
        errors,
        warnings,
        sources,
        findings,
        actions,
    })
}

fn target_dir(root: &Path, scope: SkillImportScope) -> Result<PathBuf> {
    match scope {
        SkillImportScope::Project => Ok(root.join(".jcode").join("skills")),
        SkillImportScope::Global => Ok(crate::storage::jcode_dir()?.join("skills")),
    }
}

fn default_source_dirs(root: &Path) -> Vec<PathBuf> {
    [".agents", ".claude", ".codex", ".jcode"]
        .into_iter()
        .map(|name| root.join(name).join("skills"))
        .collect()
}

fn collect_source_dir(
    source_dir: &Path,
    target_dir: &Path,
    explicit_sources: bool,
    force: bool,
    sources: &mut Vec<SkillImportSourceSummary>,
    findings: &mut Vec<SkillImportFinding>,
    actions: &mut Vec<SkillImportAction>,
) -> Result<()> {
    let origin = classify_source(source_dir);
    let exists = source_dir.is_dir();
    let mut checked = 0;

    if !exists {
        if explicit_sources {
            findings.push(finding(
                SkillImportSeverity::Warning,
                "source-missing",
                &origin,
                source_dir.display().to_string(),
                "Import source does not exist or is not a directory".to_string(),
            ));
        }
        sources.push(SkillImportSourceSummary {
            origin,
            path: source_dir.display().to_string(),
            exists,
            checked,
        });
        return Ok(());
    }

    if same_path(source_dir, target_dir) {
        findings.push(finding(
            SkillImportSeverity::Info,
            "source-is-target",
            &origin,
            source_dir.display().to_string(),
            "Import source is the selected target directory; skipping to avoid self-import"
                .to_string(),
        ));
        sources.push(SkillImportSourceSummary {
            origin,
            path: source_dir.display().to_string(),
            exists,
            checked,
        });
        return Ok(());
    }

    let mut entries = std::fs::read_dir(source_dir)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let skill_dir = entry.path();
        if !skill_dir.is_dir() {
            continue;
        }
        let skill_file = skill_dir.join("SKILL.md");
        if !skill_file.exists() {
            continue;
        }
        checked += 1;
        actions.push(plan_action(
            &origin,
            &skill_dir,
            &skill_file,
            target_dir,
            force,
        ));
    }

    sources.push(SkillImportSourceSummary {
        origin,
        path: source_dir.display().to_string(),
        exists,
        checked,
    });
    Ok(())
}

fn plan_action(
    origin: &str,
    skill_dir: &Path,
    skill_file: &Path,
    target_dir: &Path,
    force: bool,
) -> SkillImportAction {
    let parsed_name = parse_skill_name(skill_file);
    let mut action = SkillImportAction {
        name: parsed_name.as_ref().ok().cloned(),
        source_origin: origin.to_string(),
        source_path: skill_dir.display().to_string(),
        target_path: String::new(),
        action: SkillImportActionKind::Copy,
        applied: false,
        reason: None,
        findings: Vec::new(),
    };

    let name = match parsed_name {
        Ok(name) => name,
        Err(message) => {
            action.target_path = target_dir.display().to_string();
            action.action = SkillImportActionKind::SkipInvalid;
            action.reason = Some(message.clone());
            action.findings.push(finding(
                SkillImportSeverity::Warning,
                "invalid-skill-frontmatter",
                origin,
                skill_file.display().to_string(),
                message,
            ));
            return action;
        }
    };

    let target_path = target_dir.join(&name);
    action.target_path = target_path.display().to_string();

    if same_path(skill_dir, &target_path) {
        action.action = SkillImportActionKind::SkipSameTarget;
        action.reason = Some("source skill directory is already the target".to_string());
    } else if target_path.exists() && !force {
        action.action = SkillImportActionKind::SkipExisting;
        action.reason = Some(
            "target skill already exists; pass --force with --apply to overwrite files".to_string(),
        );
    } else if target_path.exists() && force {
        action.action = SkillImportActionKind::Overwrite;
    } else {
        action.action = SkillImportActionKind::Copy;
    }

    action
}

fn parse_skill_name(skill_file: &Path) -> std::result::Result<String, String> {
    let content = std::fs::read_to_string(skill_file)
        .map_err(|err| format!("Failed to read SKILL.md: {err}"))?;
    let yaml = content
        .trim_start_matches('\u{feff}')
        .trim_start()
        .strip_prefix("---")
        .and_then(|rest| rest.find("---").map(|end| &rest[..end]))
        .ok_or_else(|| "Missing or unclosed YAML frontmatter".to_string())?;
    let value: serde_yaml::Value = serde_yaml::from_str(yaml)
        .map_err(|err| format!("YAML frontmatter could not be parsed: {err}"))?;
    let map = value
        .as_mapping()
        .ok_or_else(|| "YAML frontmatter must be a mapping".to_string())?;
    let key = serde_yaml::Value::String("name".to_string());
    let name = map
        .get(&key)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "Frontmatter `name` is required".to_string())?;
    if !is_import_safe_name(name) {
        return Err(
            "Frontmatter `name` must use letters, numbers, dots, underscores, or hyphens"
                .to_string(),
        );
    }
    Ok(name.to_string())
}

fn is_import_safe_name(name: &str) -> bool {
    !name.starts_with('.')
        && !name.contains("..")
        && !name.contains('/')
        && !name.contains('\\')
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}

fn copy_skill_dir(source: &Path, target: &Path, force: bool) -> Result<()> {
    std::fs::create_dir_all(target)?;
    let mut entries = std::fs::read_dir(source)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let metadata = std::fs::symlink_metadata(&source_path)?;
        if metadata.file_type().is_symlink() {
            anyhow::bail!(
                "refusing to copy symlink inside skill directory: {}",
                source_path.display()
            );
        }
        if metadata.is_dir() {
            copy_skill_dir(&source_path, &target_path, force)?;
        } else if metadata.is_file() && (!target_path.exists() || force) {
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&source_path, &target_path)?;
        }
    }
    Ok(())
}

fn classify_source(source_dir: &Path) -> String {
    let normalized = source_dir
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>();
    for window in normalized.windows(2) {
        if window[1] == "skills" {
            return match window[0] {
                ".agents" => "agents".to_string(),
                ".claude" => "claude-compat".to_string(),
                ".codex" => "codex-compat".to_string(),
                ".jcode" => "jcode".to_string(),
                _ => "custom".to_string(),
            };
        }
    }
    "custom".to_string()
}

fn same_path(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => normalize_path(left) == normalize_path(right),
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let base = if path.is_absolute() {
        PathBuf::new()
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };
    let mut out = base;
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            _ => out.push(component.as_os_str()),
        }
    }
    out
}

fn finding(
    severity: SkillImportSeverity,
    code: impl Into<String>,
    source_origin: impl Into<String>,
    path: impl Into<String>,
    message: impl Into<String>,
) -> SkillImportFinding {
    SkillImportFinding {
        severity,
        code: code.into(),
        source_origin: source_origin.into(),
        path: path.into(),
        message: message.into(),
    }
}

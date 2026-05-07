use anyhow::Result;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillValidationStatus {
    Ok,
    Warn,
    Error,
}

impl SkillValidationStatus {
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
pub enum SkillValidationSeverity {
    Info,
    Warning,
    Error,
}

impl SkillValidationSeverity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillValidationFinding {
    pub severity: SkillValidationSeverity,
    pub code: String,
    pub origin: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillValidationOriginSummary {
    pub origin: String,
    pub path: String,
    pub exists: bool,
    pub checked: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillValidationSkill {
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub origin: String,
    pub path: String,
    pub valid: bool,
    pub effective: bool,
    pub precedence_rank: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,
    pub issues: Vec<SkillValidationFinding>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillValidationReport {
    pub status: SkillValidationStatus,
    pub offline: bool,
    pub root: String,
    pub checked: usize,
    pub valid: usize,
    pub invalid: usize,
    pub errors: usize,
    pub warnings: usize,
    pub origins: Vec<SkillValidationOriginSummary>,
    pub findings: Vec<SkillValidationFinding>,
    pub skills: Vec<SkillValidationSkill>,
}

impl SkillValidationReport {
    pub fn should_fail(&self) -> bool {
        self.errors > 0
    }
}

pub fn validate_for_working_dir(root: &Path) -> Result<SkillValidationReport> {
    let mut origins = Vec::new();
    let mut skills = Vec::new();
    let mut loose_findings = Vec::new();

    collect_builtin_skills(&mut origins, &mut skills);
    collect_skill_dir(
        &root.join(".claude").join("skills"),
        "claude-compat",
        10,
        &mut origins,
        &mut skills,
        &mut loose_findings,
    )?;

    match crate::storage::jcode_dir() {
        Ok(jcode_dir) => collect_skill_dir(
            &jcode_dir.join("skills"),
            "global",
            20,
            &mut origins,
            &mut skills,
            &mut loose_findings,
        )?,
        Err(err) => {
            origins.push(SkillValidationOriginSummary {
                origin: "global".to_string(),
                path: "<unavailable>".to_string(),
                exists: false,
                checked: 0,
            });
            loose_findings.push(finding(
                SkillValidationSeverity::Warning,
                "global-skill-dir-unavailable",
                "global",
                "<unavailable>",
                format!("Could not resolve JCODE_HOME for global skills: {err}"),
            ));
        }
    }

    collect_skill_dir(
        &root.join(".jcode").join("skills"),
        "project-local",
        30,
        &mut origins,
        &mut skills,
        &mut loose_findings,
    )?;

    Ok(finalize_report(root, origins, skills, loose_findings))
}

fn collect_builtin_skills(
    origins: &mut Vec<SkillValidationOriginSummary>,
    skills: &mut Vec<SkillValidationSkill>,
) {
    let mut checked = 0;
    for builtin in crate::skill_pack::builtin_skills() {
        checked += 1;
        let path = format!("<builtin>/{}", builtin.relative_path);
        let expected_name = Path::new(builtin.relative_path)
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str());
        skills.push(validate_skill_content(
            builtin.content,
            "built-in",
            &path,
            0,
            expected_name,
        ));
    }
    origins.push(SkillValidationOriginSummary {
        origin: "built-in".to_string(),
        path: "<builtin>/.jcode/skills".to_string(),
        exists: true,
        checked,
    });
}

fn collect_skill_dir(
    dir: &Path,
    origin: &'static str,
    precedence_rank: u8,
    origins: &mut Vec<SkillValidationOriginSummary>,
    skills: &mut Vec<SkillValidationSkill>,
    loose_findings: &mut Vec<SkillValidationFinding>,
) -> Result<()> {
    let exists = dir.is_dir();
    let mut checked = 0;

    if exists {
        let mut entries = std::fs::read_dir(dir)?.collect::<std::io::Result<Vec<_>>>()?;
        entries.sort_by_key(|entry| entry.file_name());

        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                let skill_file = path.join("SKILL.md");
                if skill_file.exists() {
                    checked += 1;
                    let expected_name = path.file_name().and_then(|name| name.to_str());
                    skills.push(validate_skill_file(
                        &skill_file,
                        origin,
                        precedence_rank,
                        expected_name,
                    ));
                } else {
                    loose_findings.push(finding(
                        SkillValidationSeverity::Warning,
                        "missing-skill-file",
                        origin,
                        path.display().to_string(),
                        "Skill directory does not contain SKILL.md".to_string(),
                    ));
                }
            } else if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".md"))
            {
                loose_findings.push(finding(
                    SkillValidationSeverity::Info,
                    "ignored-flat-skill-file",
                    origin,
                    path.display().to_string(),
                    "Skill files must live in <skills>/<name>/SKILL.md directories".to_string(),
                ));
            }
        }
    }

    origins.push(SkillValidationOriginSummary {
        origin: origin.to_string(),
        path: dir.display().to_string(),
        exists,
        checked,
    });

    Ok(())
}

fn validate_skill_file(
    path: &Path,
    origin: &'static str,
    precedence_rank: u8,
    expected_name: Option<&str>,
) -> SkillValidationSkill {
    match std::fs::read_to_string(path) {
        Ok(content) => validate_skill_content(
            &content,
            origin,
            &path.display().to_string(),
            precedence_rank,
            expected_name,
        ),
        Err(err) => {
            let mut skill = empty_skill(origin, &path.display().to_string(), precedence_rank);
            add_issue(
                &mut skill,
                SkillValidationSeverity::Error,
                "read-error",
                format!("Failed to read SKILL.md: {err}"),
            );
            skill.valid = false;
            skill
        }
    }
}

fn validate_skill_content(
    content: &str,
    origin: &'static str,
    path: &str,
    precedence_rank: u8,
    expected_name: Option<&str>,
) -> SkillValidationSkill {
    let mut skill = empty_skill(origin, path, precedence_rank);

    let (yaml, body) = match split_frontmatter(content) {
        Ok(parts) => parts,
        Err(message) => {
            add_issue(
                &mut skill,
                SkillValidationSeverity::Error,
                "frontmatter-invalid",
                message,
            );
            skill.valid = false;
            return skill;
        }
    };

    let yaml_value: serde_yaml::Value = match serde_yaml::from_str(&yaml) {
        Ok(value) => value,
        Err(err) => {
            add_issue(
                &mut skill,
                SkillValidationSeverity::Error,
                "frontmatter-yaml-invalid",
                format!("YAML frontmatter could not be parsed: {err}"),
            );
            skill.valid = false;
            return skill;
        }
    };

    let Some(map) = yaml_value.as_mapping() else {
        add_issue(
            &mut skill,
            SkillValidationSeverity::Error,
            "frontmatter-not-map",
            "YAML frontmatter must be a mapping with name and description".to_string(),
        );
        skill.valid = false;
        return skill;
    };

    skill.name = yaml_string_field(map, "name", &mut skill, true);
    skill.description = yaml_string_field(map, "description", &mut skill, true);
    skill.allowed_tools = yaml_allowed_tools(map, &mut skill);

    if let Some(name) = skill.name.clone() {
        if !is_slash_friendly_name(&name) {
            add_issue(
                &mut skill,
                SkillValidationSeverity::Warning,
                "name-not-slash-friendly",
                "Skill names should use only letters, numbers, dots, underscores, and hyphens so `/skill-name` invocation stays predictable".to_string(),
            );
        }
        if let Some(expected) = expected_name
            && name != expected
        {
            add_issue(
                &mut skill,
                SkillValidationSeverity::Warning,
                "name-directory-mismatch",
                format!(
                    "Frontmatter name `{name}` does not match containing directory `{expected}`"
                ),
            );
        }
    }

    if body.trim().is_empty() {
        add_issue(
            &mut skill,
            SkillValidationSeverity::Warning,
            "empty-body",
            "Skill body is empty; add operating guidance after the YAML frontmatter".to_string(),
        );
    }

    detect_risky_patterns(content, &mut skill);
    skill.valid = !skill
        .issues
        .iter()
        .any(|issue| issue.severity == SkillValidationSeverity::Error);
    skill
}

fn empty_skill(origin: &str, path: &str, precedence_rank: u8) -> SkillValidationSkill {
    SkillValidationSkill {
        name: None,
        description: None,
        origin: origin.to_string(),
        path: path.to_string(),
        valid: false,
        effective: false,
        precedence_rank,
        allowed_tools: None,
        issues: Vec::new(),
    }
}

fn split_frontmatter(content: &str) -> std::result::Result<(String, String), String> {
    let content = content.trim_start_matches('\u{feff}');
    let mut lines = content.lines();
    let Some(first) = lines.next() else {
        return Err("SKILL.md is empty".to_string());
    };
    if first.trim() != "---" {
        return Err("Missing YAML frontmatter delimiter `---` on the first line".to_string());
    }

    let mut yaml_lines = Vec::new();
    let mut body_lines = Vec::new();
    let mut in_yaml = true;
    for line in lines {
        if in_yaml && (line.trim() == "---" || line.trim() == "...") {
            in_yaml = false;
            continue;
        }
        if in_yaml {
            yaml_lines.push(line);
        } else {
            body_lines.push(line);
        }
    }

    if in_yaml {
        return Err("Unclosed YAML frontmatter; add a closing `---` line".to_string());
    }

    Ok((yaml_lines.join("\n"), body_lines.join("\n")))
}

fn yaml_string_field(
    map: &serde_yaml::Mapping,
    key: &str,
    skill: &mut SkillValidationSkill,
    required: bool,
) -> Option<String> {
    let key_value = serde_yaml::Value::String(key.to_string());
    match map.get(&key_value) {
        Some(serde_yaml::Value::String(value)) => {
            let value = value.trim();
            if value.is_empty() {
                add_issue(
                    skill,
                    SkillValidationSeverity::Error,
                    format!("empty-{key}"),
                    format!("Frontmatter `{key}` must not be empty"),
                );
                None
            } else {
                Some(value.to_string())
            }
        }
        Some(_) => {
            add_issue(
                skill,
                SkillValidationSeverity::Error,
                format!("invalid-{key}"),
                format!("Frontmatter `{key}` must be a string"),
            );
            None
        }
        None if required => {
            add_issue(
                skill,
                SkillValidationSeverity::Error,
                format!("missing-{key}"),
                format!("Frontmatter `{key}` is required"),
            );
            None
        }
        None => None,
    }
}

fn yaml_allowed_tools(
    map: &serde_yaml::Mapping,
    skill: &mut SkillValidationSkill,
) -> Option<Vec<String>> {
    let key_value = serde_yaml::Value::String("allowed-tools".to_string());
    let value = map.get(&key_value)?;
    match value {
        serde_yaml::Value::String(raw) => {
            let tools = raw
                .split(',')
                .map(str::trim)
                .filter(|tool| !tool.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>();
            if tools.is_empty() {
                add_issue(
                    skill,
                    SkillValidationSeverity::Warning,
                    "empty-allowed-tools",
                    "`allowed-tools` is present but does not list any tools".to_string(),
                );
                None
            } else {
                Some(tools)
            }
        }
        serde_yaml::Value::Sequence(values) => {
            let mut tools = Vec::new();
            for value in values {
                match value {
                    serde_yaml::Value::String(tool) => {
                        let tool = tool.trim();
                        if !tool.is_empty() {
                            tools.push(tool.to_string());
                        }
                    }
                    _ => add_issue(
                        skill,
                        SkillValidationSeverity::Error,
                        "invalid-allowed-tools",
                        "`allowed-tools` list entries must be strings".to_string(),
                    ),
                }
            }
            if tools.is_empty() {
                add_issue(
                    skill,
                    SkillValidationSeverity::Warning,
                    "empty-allowed-tools",
                    "`allowed-tools` is present but does not list any tools".to_string(),
                );
                None
            } else {
                Some(tools)
            }
        }
        serde_yaml::Value::Null => {
            add_issue(
                skill,
                SkillValidationSeverity::Warning,
                "empty-allowed-tools",
                "`allowed-tools` is null; omit it or use a comma-separated string/list".to_string(),
            );
            None
        }
        _ => {
            add_issue(
                skill,
                SkillValidationSeverity::Error,
                "invalid-allowed-tools",
                "`allowed-tools` must be a comma-separated string or list of strings in the current jcode skill runtime".to_string(),
            );
            None
        }
    }
}

fn is_slash_friendly_name(name: &str) -> bool {
    !name.starts_with('.')
        && !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}

fn detect_risky_patterns(content: &str, skill: &mut SkillValidationSkill) {
    let lower = content.to_ascii_lowercase();
    let collapsed = lower.split_whitespace().collect::<Vec<_>>().join(" ");

    if collapsed.contains("ignore previous instructions")
        || collapsed.contains("disregard previous instructions")
    {
        add_issue(
            skill,
            SkillValidationSeverity::Warning,
            "prompt-injection-phrase",
            "Skill contains a common prompt-injection phrase; review before enabling it"
                .to_string(),
        );
    }

    if collapsed.contains("rm -rf /") || collapsed.contains("rm -rf /*") {
        add_issue(
            skill,
            SkillValidationSeverity::Warning,
            "dangerous-shell-command",
            "Skill mentions `rm -rf /`; require human review before use".to_string(),
        );
    }

    if (collapsed.contains("curl ") || collapsed.contains("wget "))
        && (collapsed.contains("| sh") || collapsed.contains("| bash"))
    {
        add_issue(
            skill,
            SkillValidationSeverity::Warning,
            "pipe-to-shell",
            "Skill appears to pipe a network download into a shell".to_string(),
        );
    }

    if collapsed.contains("chmod -r 777") {
        add_issue(
            skill,
            SkillValidationSeverity::Warning,
            "unsafe-permissions-command",
            "Skill mentions recursive world-writable permissions".to_string(),
        );
    }

    for line in content.lines() {
        if looks_like_secret_assignment(line) {
            add_issue(
                skill,
                SkillValidationSeverity::Warning,
                "possible-secret",
                "Skill appears to contain an inline secret assignment; redact before committing"
                    .to_string(),
            );
            break;
        }
    }
}

fn looks_like_secret_assignment(line: &str) -> bool {
    let lower = line.trim().to_ascii_lowercase();
    if lower.contains("placeholder")
        || lower.contains("example")
        || lower.contains("your_")
        || lower.contains("<token")
        || lower.contains("<secret")
    {
        return false;
    }

    let has_secret_key = [
        "api_key",
        "apikey",
        "access_token",
        "auth_token",
        "secret_key",
        "password",
    ]
    .iter()
    .any(|needle| lower.contains(needle));
    if !has_secret_key || !(lower.contains(':') || lower.contains('=')) {
        return false;
    }

    lower
        .split([':', '='])
        .nth(1)
        .map(|value| {
            value
                .chars()
                .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
                .count()
                >= 12
        })
        .unwrap_or(false)
}

fn finalize_report(
    root: &Path,
    origins: Vec<SkillValidationOriginSummary>,
    mut skills: Vec<SkillValidationSkill>,
    mut findings: Vec<SkillValidationFinding>,
) -> SkillValidationReport {
    mark_effective_skills(&mut skills);

    for skill in &skills {
        findings.extend(skill.issues.iter().cloned());
    }

    let errors = findings
        .iter()
        .filter(|finding| finding.severity == SkillValidationSeverity::Error)
        .count();
    let warnings = findings
        .iter()
        .filter(|finding| finding.severity == SkillValidationSeverity::Warning)
        .count();
    let checked = skills.len();
    let valid = skills.iter().filter(|skill| skill.valid).count();
    let invalid = checked.saturating_sub(valid);
    let status = if errors > 0 {
        SkillValidationStatus::Error
    } else if warnings > 0 {
        SkillValidationStatus::Warn
    } else {
        SkillValidationStatus::Ok
    };

    SkillValidationReport {
        status,
        offline: true,
        root: root.display().to_string(),
        checked,
        valid,
        invalid,
        errors,
        warnings,
        origins,
        findings,
        skills,
    }
}

fn mark_effective_skills(skills: &mut [SkillValidationSkill]) {
    let mut by_name: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (idx, skill) in skills.iter().enumerate() {
        if skill.valid
            && let Some(name) = &skill.name
        {
            by_name.entry(name.clone()).or_default().push(idx);
        }
    }

    for (name, indices) in by_name {
        let max_rank = indices
            .iter()
            .map(|idx| skills[*idx].precedence_rank)
            .max()
            .unwrap_or(0);
        let max_indices = indices
            .iter()
            .copied()
            .filter(|idx| skills[*idx].precedence_rank == max_rank)
            .collect::<Vec<_>>();

        if max_indices.len() == 1 {
            skills[max_indices[0]].effective = true;
        } else {
            for idx in &max_indices {
                let paths = max_indices
                    .iter()
                    .map(|peer| skills[*peer].path.clone())
                    .collect::<Vec<_>>()
                    .join(", ");
                add_issue(
                    &mut skills[*idx],
                    SkillValidationSeverity::Warning,
                    "same-precedence-duplicate",
                    format!(
                        "Skill `{name}` has multiple definitions at the same precedence; runtime winner may depend on filesystem order: {paths}"
                    ),
                );
            }
        }

        for idx in indices {
            if skills[idx].precedence_rank < max_rank {
                add_issue(
                    &mut skills[idx],
                    SkillValidationSeverity::Info,
                    "shadowed-skill",
                    format!("Skill `{name}` is shadowed by a higher-precedence definition"),
                );
            }
        }
    }
}

fn add_issue(
    skill: &mut SkillValidationSkill,
    severity: SkillValidationSeverity,
    code: impl Into<String>,
    message: impl Into<String>,
) {
    skill.issues.push(finding(
        severity,
        code,
        &skill.origin,
        &skill.path,
        message.into(),
    ));
}

fn finding(
    severity: SkillValidationSeverity,
    code: impl Into<String>,
    origin: impl Into<String>,
    path: impl Into<String>,
    message: impl Into<String>,
) -> SkillValidationFinding {
    SkillValidationFinding {
        severity,
        code: code.into(),
        origin: origin.into(),
        path: path.into(),
        message: message.into(),
    }
}

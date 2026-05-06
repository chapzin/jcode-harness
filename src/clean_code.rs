use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

pub const BUILTIN_RULES_YAML: &str = include_str!("../.jcode/quality/clean-code-rules.yaml");

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::Info => 0,
            Self::Warning => 1,
            Self::Error => 2,
        }
    }
}

impl Ord for Severity {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank().cmp(&other.rank())
    }
}

impl PartialOrd for Severity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl std::str::FromStr for Severity {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "info" => Ok(Self::Info),
            "warning" | "warn" => Ok(Self::Warning),
            "error" | "err" => Ok(Self::Error),
            other => anyhow::bail!("invalid severity '{other}', expected info, warning, or error"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CleanCodeRulePack {
    pub version: u32,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub severity_levels: Vec<String>,
    #[serde(default)]
    pub rules: Vec<CleanCodeRule>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CleanCodeRule {
    pub id: String,
    pub title: String,
    pub severity: Severity,
    pub category: String,
    pub check_type: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct CleanCodeReport {
    pub root: PathBuf,
    pub files_scanned: usize,
    pub findings: Vec<CleanCodeFinding>,
    pub rules_loaded: usize,
}

impl CleanCodeReport {
    pub fn error_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == Severity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == Severity::Warning)
            .count()
    }

    pub fn info_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == Severity::Info)
            .count()
    }

    pub fn has_at_least(&self, severity: Severity) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.severity >= severity)
    }
}

#[derive(Debug, Serialize)]
pub struct CleanCodeFinding {
    pub rule_id: String,
    pub severity: Severity,
    pub path: PathBuf,
    pub line: usize,
    pub message: String,
    pub snippet: String,
}

#[derive(Debug, Clone)]
pub struct CleanCodeCheckOptions {
    pub root: PathBuf,
    pub paths: Vec<PathBuf>,
}

pub fn load_rule_pack(path: Option<&Path>) -> Result<CleanCodeRulePack> {
    let content = if let Some(path) = path {
        fs::read_to_string(path)?
    } else {
        BUILTIN_RULES_YAML.to_string()
    };
    Ok(serde_yaml::from_str(&content)?)
}

pub fn check(options: CleanCodeCheckOptions) -> Result<CleanCodeReport> {
    let rules = load_rule_pack(None)?;
    let targets = if options.paths.is_empty() {
        vec![options.root.clone()]
    } else {
        options
            .paths
            .iter()
            .map(|path| {
                if path.is_absolute() {
                    path.clone()
                } else {
                    options.root.join(path)
                }
            })
            .collect()
    };

    let mut files = Vec::new();
    for target in targets {
        collect_files(&target, &mut files)?;
    }
    files.sort();
    files.dedup();

    let mut findings = Vec::new();
    let mut files_scanned = 0;
    for file in files {
        if !is_supported_file(&file) {
            continue;
        }
        let Ok(content) = fs::read_to_string(&file) else {
            continue;
        };
        files_scanned += 1;
        scan_file(&options.root, &file, &content, &mut findings);
    }

    Ok(CleanCodeReport {
        root: options.root,
        files_scanned,
        findings,
        rules_loaded: rules.rules.len(),
    })
}

fn collect_files(path: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(()),
    };
    if metadata.is_file() {
        out.push(path.to_path_buf());
        return Ok(());
    }
    if !metadata.is_dir() || should_skip_dir(path) {
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if !should_skip_dir(&path) {
                collect_files(&path, out)?;
            }
        } else {
            out.push(path);
        }
    }
    Ok(())
}

fn should_skip_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    matches!(
        name,
        ".git"
            | "target"
            | "node_modules"
            | ".next"
            | "dist"
            | "build"
            | "vendor"
            | ".venv"
            | "__pycache__"
    )
}

fn is_supported_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some(
            "rs" | "js"
                | "jsx"
                | "ts"
                | "tsx"
                | "py"
                | "go"
                | "java"
                | "kt"
                | "swift"
                | "c"
                | "cc"
                | "cpp"
                | "h"
                | "hpp"
        )
    )
}

fn scan_file(root: &Path, path: &Path, content: &str, findings: &mut Vec<CleanCodeFinding>) {
    let lines: Vec<&str> = content.lines().collect();
    let display_path = path.strip_prefix(root).unwrap_or(path).to_path_buf();

    if lines.len() > 1000 {
        findings.push(CleanCodeFinding {
            rule_id: "manageable-file-size".into(),
            severity: Severity::Error,
            path: display_path.clone(),
            line: 1,
            message: format!("file is very large: {} lines", lines.len()),
            snippet: String::new(),
        });
    } else if lines.len() > 500 {
        findings.push(CleanCodeFinding {
            rule_id: "manageable-file-size".into(),
            severity: Severity::Warning,
            path: display_path.clone(),
            line: 1,
            message: format!("file is large: {} lines", lines.len()),
            snippet: String::new(),
        });
    }

    scan_function_lengths(&display_path, &lines, findings);
    scan_silent_errors(&display_path, &lines, findings);
    scan_long_lines(&display_path, &lines, findings);
}

fn scan_function_lengths(path: &Path, lines: &[&str], findings: &mut Vec<CleanCodeFinding>) {
    let mut function_start: Option<(usize, String)> = None;
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if looks_like_function_start(trimmed) {
            if let Some((start, signature)) = function_start.take() {
                push_function_length(path, start, idx, &signature, findings);
            }
            function_start = Some((idx + 1, trimmed.to_string()));
        }
    }
    if let Some((start, signature)) = function_start {
        push_function_length(path, start, lines.len(), &signature, findings);
    }
}

fn looks_like_function_start(trimmed: &str) -> bool {
    trimmed.starts_with("fn ")
        || trimmed.starts_with("pub fn ")
        || trimmed.starts_with("pub(crate) fn ")
        || trimmed.starts_with("async fn ")
        || trimmed.starts_with("pub async fn ")
        || trimmed.starts_with("def ")
        || trimmed.starts_with("function ")
        || trimmed.contains(" function ")
}

fn push_function_length(
    path: &Path,
    start: usize,
    end_exclusive: usize,
    signature: &str,
    findings: &mut Vec<CleanCodeFinding>,
) {
    let len = end_exclusive.saturating_sub(start).max(1);
    let severity = if len > 80 {
        Severity::Error
    } else if len > 40 {
        Severity::Warning
    } else {
        return;
    };
    findings.push(CleanCodeFinding {
        rule_id: "small-focused-functions".into(),
        severity,
        path: path.to_path_buf(),
        line: start,
        message: format!("function-like block is {len} lines; consider extracting focused steps"),
        snippet: signature.chars().take(140).collect(),
    });
}

fn scan_silent_errors(path: &Path, lines: &[&str], findings: &mut Vec<CleanCodeFinding>) {
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let suspicious = trimmed == "except: pass"
            || trimmed.starts_with("except ") && trimmed.ends_with(": pass")
            || trimmed == "catch {}"
            || trimmed == "catch (e) {}"
            || trimmed == "catch (err) {}"
            || trimmed.contains("Err(_) => {}") // clean-code: allow analyzer pattern literal
            || trimmed.contains("Err(_) => { }") // clean-code: allow analyzer pattern literal
            || trimmed.starts_with("let _ =")
            || trimmed.contains(".ok();"); // clean-code: allow analyzer pattern literal
        if suspicious && !trimmed.contains("clean-code: allow") {
            findings.push(CleanCodeFinding {
                rule_id: "no-silent-error-swallowing".into(),
                severity: Severity::Error,
                path: path.to_path_buf(),
                line: idx + 1,
                message:
                    "possible silently ignored error; handle, propagate, log, or document intent"
                        .into(),
                snippet: trimmed.chars().take(140).collect(),
            });
        }
    }
}

fn scan_long_lines(path: &Path, lines: &[&str], findings: &mut Vec<CleanCodeFinding>) {
    for (idx, line) in lines.iter().enumerate() {
        if line.chars().count() > 140 {
            findings.push(CleanCodeFinding {
                rule_id: "readable-names".into(),
                severity: Severity::Info,
                path: path.to_path_buf(),
                line: idx + 1,
                message: "line is longer than 140 characters; check readability".into(),
                snippet: line.trim().chars().take(140).collect(),
            });
        }
    }
}

pub fn print_human_report(report: &CleanCodeReport) {
    println!("Clean Code Guardian report");
    println!("root: {}", report.root.display());
    println!("files scanned: {}", report.files_scanned);
    println!(
        "findings: {} error(s), {} warning(s), {} info",
        report.error_count(),
        report.warning_count(),
        report.info_count()
    );
    if report.findings.is_empty() {
        println!("status: pass");
        return;
    }
    println!();
    for finding in &report.findings {
        println!(
            "{}:{} [{}] {}: {}",
            finding.path.display(),
            finding.line,
            finding.severity.as_str(),
            finding.rule_id,
            finding.message
        );
        if !finding.snippet.is_empty() {
            println!("  {}", finding.snippet);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_builtin_rules() {
        let pack = load_rule_pack(None).unwrap();
        assert_eq!(pack.name, "clean-code-default");
        assert!(
            pack.rules
                .iter()
                .any(|rule| rule.id == "small-focused-functions")
        );
    }

    #[test]
    fn detects_silent_error_and_long_function() {
        let dir =
            std::env::temp_dir().join(format!("jcode-clean-code-test-{}", crate::id::new_id("t")));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("main.rs");
        let mut content = String::from("fn long_one() {\n");
        for _ in 0..45 {
            content.push_str("    println!(\"x\");\n");
        }
        content
            .push_str("}\nfn ignore() {\n    let _ = std::fs::read_to_string(\"missing\");\n}\n");
        fs::write(&file, content).unwrap();
        let report = check(CleanCodeCheckOptions {
            root: dir.clone(),
            paths: vec![],
        })
        .unwrap();
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.rule_id == "small-focused-functions")
        );
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.rule_id == "no-silent-error-swallowing")
        );
        if fs::remove_dir_all(dir).is_err() {
            // Best-effort test cleanup only.
        }
    }
}

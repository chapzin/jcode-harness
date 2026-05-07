use anyhow::Result;
use chrono::Utc;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MemoryBackend {
    Legacy,
    Wiki,
    Hybrid,
    Off,
}

impl MemoryBackend {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "legacy" => Some(Self::Legacy),
            "wiki" | "llm-wiki" => Some(Self::Wiki),
            "hybrid" => Some(Self::Hybrid),
            "off" | "disabled" | "none" => Some(Self::Off),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Legacy => "legacy",
            Self::Wiki => "wiki",
            Self::Hybrid => "hybrid",
            Self::Off => "off",
        }
    }

    pub fn uses_legacy(self) -> bool {
        matches!(self, Self::Legacy | Self::Hybrid)
    }

    pub fn uses_wiki(self) -> bool {
        matches!(self, Self::Wiki | Self::Hybrid)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WikiScope {
    GlobalCache,
    RepoLocal,
}

impl WikiScope {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "global-cache" | "global" => Some(Self::GlobalCache),
            "repo-local" | "local" | "project" => Some(Self::RepoLocal),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::GlobalCache => "global-cache",
            Self::RepoLocal => "repo-local",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WikiStatus {
    pub backend: MemoryBackend,
    pub scope: WikiScope,
    pub root: PathBuf,
    pub exists: bool,
    pub schema_exists: bool,
    pub index_exists: bool,
    pub overview_exists: bool,
    pub log_exists: bool,
}

pub const SCHEMA_SUMMARY: &str = "LLM Wiki memory stores immutable raw sources under raw/ and maintained Markdown pages under wiki/. Keep pages auditable, cite raw sources in frontmatter or inline, use wikilinks [[...]], mark uncertainty, log important edits, and never store secrets, tokens, passwords, .env contents, private keys, or sensitive personal data.";

pub const SCHEMA_FULL: &str = r#"# Jcode Living Memory Schema

This memory backend follows the LLM Wiki pattern: immutable raw sources plus a maintained Markdown wiki.

## Rules

1. Raw sources under `raw/` are append-only source material. Read them, cite them, but do not rewrite them unless the user explicitly requests maintenance/redaction.
2. Wiki pages under `wiki/` are working memory. Update them when durable user preferences, project facts, decisions, procedures, conventions, entities, concepts, or recurring issues are learned.
3. Every important claim should cite provenance using `sources` frontmatter and/or inline references to `raw/...` paths.
4. Use YAML frontmatter for all wiki pages: `title`, `kind`, `scope`, `created_at`, `updated_at`, `sources`, `confidence`, and `status`.
5. Use wikilinks like `[[user/preferences]]` and `[[projects/<project_slug>/overview]]` for related pages.
6. Preserve conflicts. Do not silently overwrite contradictory information. Add notes to `open_questions.md` or a page section named `Conflicts`.
7. Keep `index.md` useful for navigation and `log.md` useful for audit.
8. Never store secrets, tokens, passwords, `.env` contents, private keys, credentials, or sensitive personal data.
9. Prefer concise, stable summaries over chat transcript dumps. Raw sources are for transcript-level detail.
10. Offline operation is required. Do not depend on network access to read or update the wiki.
"#;

pub fn configured_backend() -> MemoryBackend {
    if let Ok(value) = std::env::var("JCODE_MEMORY_BACKEND")
        && let Some(backend) = MemoryBackend::parse(&value)
    {
        return backend;
    }
    MemoryBackend::parse(&crate::config::config().memory.backend).unwrap_or(MemoryBackend::Legacy)
}

pub fn configured_scope() -> WikiScope {
    if let Ok(value) = std::env::var("JCODE_MEMORY_WIKI_SCOPE")
        && let Some(scope) = WikiScope::parse(&value)
    {
        return scope;
    }
    WikiScope::parse(&crate::config::config().memory.wiki_scope).unwrap_or(WikiScope::GlobalCache)
}

pub fn root_for_scope(scope: WikiScope, working_dir: Option<&Path>) -> Result<PathBuf> {
    match scope {
        WikiScope::GlobalCache => Ok(crate::storage::jcode_dir()?.join("memory_wiki")),
        WikiScope::RepoLocal => Ok(working_dir
            .map(Path::to_path_buf)
            .unwrap_or(std::env::current_dir()?)
            .join(".jcode/memory_wiki")),
    }
}

pub fn current_root(working_dir: Option<&Path>) -> Result<PathBuf> {
    root_for_scope(configured_scope(), working_dir)
}

pub fn ensure_layout(working_dir: Option<&Path>) -> Result<PathBuf> {
    let root = current_root(working_dir)?;
    ensure_layout_at(&root)?;
    Ok(root)
}

pub fn ensure_layout_at(root: &Path) -> Result<()> {
    for dir in [
        "raw/sessions",
        "raw/imports",
        "raw/artifacts",
        "raw/manual",
        "wiki/user",
        "wiki/projects",
        "wiki/entities",
        "wiki/concepts",
        "wiki/sources",
        "cache",
        "trash",
    ] {
        std::fs::create_dir_all(root.join(dir))?;
    }

    write_if_missing(root.join("schema.md"), SCHEMA_FULL.to_string())?;
    write_if_missing(root.join("index.md"), default_index())?;
    write_if_missing(root.join("overview.md"), default_overview())?;
    write_if_missing(root.join("log.md"), default_log())?;
    write_if_missing(
        root.join("wiki/user/preferences.md"),
        default_page("Preferências do usuário", "user-preferences", "global"),
    )?;
    write_if_missing(
        root.join("wiki/user/corrections.md"),
        default_page("Correções do usuário", "user-corrections", "global"),
    )?;
    write_if_missing(
        root.join("wiki/user/procedures.md"),
        default_page("Procedimentos do usuário", "user-procedures", "global"),
    )?;
    write_if_missing(
        root.join("wiki/user/style.md"),
        default_page("Estilo do usuário", "user-style", "global"),
    )?;
    write_if_missing(root.join("cache/search_index.json"), "{}\n".to_string())?;
    write_if_missing(root.join("cache/page_manifest.json"), "{}\n".to_string())?;

    Ok(())
}

pub fn status(working_dir: Option<&Path>) -> Result<WikiStatus> {
    let backend = configured_backend();
    let scope = configured_scope();
    let root = root_for_scope(scope, working_dir)?;
    Ok(WikiStatus {
        backend,
        scope,
        exists: root.exists(),
        schema_exists: root.join("schema.md").exists(),
        index_exists: root.join("index.md").exists(),
        overview_exists: root.join("overview.md").exists(),
        log_exists: root.join("log.md").exists(),
        root,
    })
}

pub fn build_prompt(working_dir: Option<&Path>, max_chars: usize) -> Result<Option<String>> {
    let root = current_root(working_dir)?;
    if !root.exists() {
        return Ok(None);
    }

    let mut sections = Vec::new();
    sections.push(format!("# Jcode Living Memory\n\n{}", SCHEMA_SUMMARY));
    for rel in [
        "overview.md",
        "index.md",
        "wiki/user/preferences.md",
        "wiki/user/corrections.md",
        "wiki/user/procedures.md",
        "wiki/user/style.md",
    ] {
        let path = root.join(rel);
        if let Ok(content) = std::fs::read_to_string(&path) {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                sections.push(format!("## {}\n\n{}", rel, trimmed));
            }
        }
    }

    let mut prompt = sections.join("\n\n");
    if prompt.chars().count() > max_chars {
        prompt = prompt.chars().take(max_chars).collect();
        prompt.push_str("\n\n[Living Memory truncated]");
    }
    Ok(Some(prompt))
}

pub fn search(query: &str, working_dir: Option<&Path>) -> Result<Vec<(PathBuf, String)>> {
    let root = current_root(working_dir)?;
    if !root.exists() {
        return Ok(Vec::new());
    }
    let needle = query.to_lowercase();
    let mut hits = Vec::new();
    search_dir(&root, &root, &needle, &mut hits)?;
    Ok(hits)
}

fn search_dir(
    root: &Path,
    dir: &Path,
    needle: &str,
    hits: &mut Vec<(PathBuf, String)>,
) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|n| n.to_str()) == Some("trash") {
                continue;
            }
            search_dir(root, &path, needle, hits)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let content = std::fs::read_to_string(&path)?;
            if content.to_lowercase().contains(needle) {
                let snippet = content
                    .lines()
                    .find(|line| line.to_lowercase().contains(needle))
                    .unwrap_or_else(|| content.lines().next().unwrap_or(""))
                    .trim()
                    .to_string();
                hits.push((
                    path.strip_prefix(root).unwrap_or(&path).to_path_buf(),
                    snippet,
                ));
            }
        }
    }
    Ok(())
}

fn write_if_missing(path: PathBuf, content: String) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn default_index() -> String {
    format!(
        "# Jcode Living Memory Index\n\nCreated: {}\n\n- [[overview]]\n- [[wiki/user/preferences]]\n- [[wiki/user/corrections]]\n- [[wiki/user/procedures]]\n- [[wiki/user/style]]\n\n",
        now()
    )
}

fn default_overview() -> String {
    format!(
        "---\ntitle: \"Living Memory Overview\"\nkind: \"overview\"\nscope: \"global\"\ncreated_at: \"{}\"\nupdated_at: \"{}\"\nsources: []\nconfidence: 1.0\nstatus: \"active\"\n---\n\n# Living Memory Overview\n\nThis wiki is initialized but does not yet contain curated knowledge.\n",
        now(),
        now()
    )
}

fn default_log() -> String {
    format!(
        "# Living Memory Log\n\n- {} initialized wiki layout.\n",
        now()
    )
}

fn default_page(title: &str, kind: &str, scope: &str) -> String {
    format!(
        "---\ntitle: \"{}\"\nkind: \"{}\"\nscope: \"{}\"\ncreated_at: \"{}\"\nupdated_at: \"{}\"\nsources: []\nconfidence: 0.0\nstatus: \"active\"\n---\n\n# {}\n\nNo durable entries yet.\n",
        title,
        kind,
        scope,
        now(),
        now(),
        title
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_layout_creates_markdown_wiki() {
        let temp = tempfile::tempdir().expect("tempdir");
        ensure_layout_at(temp.path()).expect("layout");
        assert!(temp.path().join("schema.md").exists());
        assert!(temp.path().join("raw/sessions").is_dir());
        assert!(temp.path().join("wiki/user/preferences.md").exists());
    }

    #[test]
    fn backend_parser_accepts_expected_values() {
        assert_eq!(MemoryBackend::parse("legacy"), Some(MemoryBackend::Legacy));
        assert_eq!(MemoryBackend::parse("llm-wiki"), Some(MemoryBackend::Wiki));
        assert_eq!(MemoryBackend::parse("hybrid"), Some(MemoryBackend::Hybrid));
        assert_eq!(MemoryBackend::parse("off"), Some(MemoryBackend::Off));
    }
}

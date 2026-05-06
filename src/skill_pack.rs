use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
pub struct BuiltinSkill {
    pub name: &'static str,
    pub relative_path: &'static str,
    pub content: &'static str,
}

pub const BUILTIN_SKILLS: &[BuiltinSkill] = &[
    BuiltinSkill {
        name: "karpathy-guidelines",
        relative_path: ".jcode/skills/karpathy-guidelines/SKILL.md",
        content: include_str!("../.jcode/skills/karpathy-guidelines/SKILL.md"),
    },
    BuiltinSkill {
        name: "optimization",
        relative_path: ".jcode/skills/optimization/SKILL.md",
        content: include_str!("../.jcode/skills/optimization/SKILL.md"),
    },
    BuiltinSkill {
        name: "clean-code-guardian",
        relative_path: ".jcode/skills/clean-code-guardian/SKILL.md",
        content: include_str!("../.jcode/skills/clean-code-guardian/SKILL.md"),
    },
    BuiltinSkill {
        name: "llmwiki-memory",
        relative_path: ".jcode/skills/llmwiki-memory/SKILL.md",
        content: include_str!("../.jcode/skills/llmwiki-memory/SKILL.md"),
    },
];

pub fn builtin_skills() -> &'static [BuiltinSkill] {
    BUILTIN_SKILLS
}

pub fn builtin_skill(name: &str) -> Option<&'static BuiltinSkill> {
    BUILTIN_SKILLS.iter().find(|skill| skill.name == name)
}

pub fn sync_builtin_skills(force: bool) -> Result<Vec<PathBuf>> {
    let skills_dir = crate::storage::jcode_dir()?.join("skills");
    let mut written = Vec::new();

    for skill in BUILTIN_SKILLS {
        let path = skills_dir.join(skill.name).join("SKILL.md");
        if path.exists() && !force {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, skill.content)?;
        written.push(path);
    }

    Ok(written)
}

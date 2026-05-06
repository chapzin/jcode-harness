#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SkillMode {
    Auto,
    Off,
    Always,
}

pub fn select_skills(goal: &str, explicit: &[String], mode: SkillMode) -> Vec<String> {
    if mode == SkillMode::Off {
        return explicit.to_vec();
    }

    let text = goal.to_lowercase();
    let mut selected = Vec::new();
    for name in explicit {
        push_unique(&mut selected, name);
    }

    let coding_terms = [
        "código",
        "codigo",
        "code",
        "bug",
        "teste",
        "test",
        "refator",
        "review",
        "revisar",
        "implementar",
        "implement",
        "corrigir",
        "fix",
        "otimizar",
        "pr",
        "diff",
    ];
    let perf_terms = [
        "performance",
        "latência",
        "latencia",
        "memória",
        "memoria",
        "throughput",
        "eficiência",
        "efficiency",
        "cpu",
        "ram",
    ];

    if mode == SkillMode::Always || coding_terms.iter().any(|term| text.contains(term)) {
        push_unique(&mut selected, "karpathy-guidelines");
    }
    if mode == SkillMode::Always || perf_terms.iter().any(|term| text.contains(term)) {
        push_unique(&mut selected, "optimization");
    }

    selected
}

pub fn build_skill_preface(goal: &str, explicit: &[String], mode: SkillMode) -> Option<String> {
    let selected = select_skills(goal, explicit, mode);
    if selected.is_empty() {
        return None;
    }

    let registry = crate::skill::SkillRegistry::load().ok()?;
    let mut out =
        String::from("Use these selected skills for this task. Do not load unrelated skills.\n");
    for name in selected {
        if let Some(skill) = registry.get(&name) {
            out.push_str(&format!(
                "\n## Skill: {}\n{}\n",
                skill.name, skill.description
            ));
            if name == "karpathy-guidelines" {
                out.push_str("Minimal principles: think before coding; make surgical changes; avoid speculative abstractions; state assumptions; define verifiable success criteria.\n");
            }
            out.push_str(&skill.content);
            out.push('\n');
        }
    }

    Some(out)
}

fn push_unique(selected: &mut Vec<String>, name: &str) {
    if !selected.iter().any(|existing| existing == name) {
        selected.push(name.to_string());
    }
}

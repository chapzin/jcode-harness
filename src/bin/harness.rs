use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use jcode::id::new_id;
use jcode::message::{Message, ToolDefinition};
use jcode::provider::{EventStream, Provider};
use jcode::tool::{Registry, ToolContext, ToolExecutionMode};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "jcode-harness")]
#[command(about = "Standalone jcode harness utilities")]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the deterministic tool harness smoke test
    Smoke(SmokeArgs),
    /// Run a single goal by delegating to the jcode run path with skill routing
    Run(RunArgs),
    /// Manage embedded and local skills
    Skills(SkillsArgs),
}

#[derive(Parser, Clone)]
struct SmokeArgs {
    /// Use an explicit working directory (defaults to a temp folder).
    #[arg(long)]
    cwd: Option<String>,

    /// Include network-backed tools (webfetch/websearch/codesearch).
    #[arg(long)]
    include_network: bool,
}

#[derive(Parser)]
struct SkillsArgs {
    #[command(subcommand)]
    command: SkillsCommand,
}

#[derive(Subcommand)]
enum SkillsCommand {
    List,
    Show {
        name: String,
    },
    Sync {
        #[arg(long)]
        force: bool,
    },
    Doctor,
}

#[derive(Parser)]
struct RunArgs {
    goal: String,
    #[arg(long)]
    cwd: Option<String>,
    #[arg(long)]
    provider: Option<String>,
    #[arg(long)]
    provider_profile: Option<String>,
    #[arg(long)]
    model: Option<String>,
    #[arg(long, default_value = "auto")]
    skills: HarnessSkillMode,
    #[arg(long = "skill")]
    skill: Vec<String>,
    #[arg(long)]
    max_turns: Option<usize>,
    #[arg(long, conflicts_with = "ndjson")]
    json: bool,
    #[arg(long, conflicts_with = "json")]
    ndjson: bool,
    #[arg(long)]
    dry_run: bool,
}

#[derive(Clone, ValueEnum)]
enum HarnessSkillMode {
    Auto,
    Off,
    Always,
}

impl From<HarnessSkillMode> for jcode::skill_router::SkillMode {
    fn from(value: HarnessSkillMode) -> Self {
        match value {
            HarnessSkillMode::Auto => Self::Auto,
            HarnessSkillMode::Off => Self::Off,
            HarnessSkillMode::Always => Self::Always,
        }
    }
}

struct NoopProvider;

#[async_trait::async_trait]
impl Provider for NoopProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolDefinition],
        _system: &str,
        _resume_session_id: Option<&str>,
    ) -> Result<EventStream> {
        anyhow::bail!("Noop provider - tool harness does not invoke models.")
    }

    fn name(&self) -> &str {
        "noop"
    }
    fn fork(&self) -> Arc<dyn Provider> {
        Arc::new(NoopProvider)
    }
    fn available_models_display(&self) -> Vec<String> {
        vec![]
    }
    async fn prefetch_models(&self) -> Result<()> {
        Ok(())
    }
}

struct ToolCase {
    name: &'static str,
    input: serde_json::Value,
    label: &'static str,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    match args.command.unwrap_or(Command::Smoke(SmokeArgs {
        cwd: None,
        include_network: false,
    })) {
        Command::Smoke(args) => run_smoke(args).await,
        Command::Run(args) => run_goal(args),
        Command::Skills(args) => run_skills(args),
    }
}

fn run_goal(args: RunArgs) -> Result<()> {
    let message =
        jcode::cli::commands::with_auto_skill_preface(&args.goal, &args.skill, args.skills.into());
    if args.dry_run {
        println!("{}", message);
        return Ok(());
    }

    let mut cmd = std::process::Command::new(jcode_binary_path());
    if let Some(cwd) = args.cwd {
        cmd.arg("--cwd").arg(cwd);
    }
    if let Some(provider) = args.provider {
        cmd.arg("--provider").arg(provider);
    }
    if let Some(profile) = args.provider_profile {
        cmd.arg("--provider-profile").arg(profile);
    }
    if let Some(model) = args.model {
        cmd.arg("--model").arg(model);
    }
    if let Some(max_turns) = args.max_turns {
        cmd.env("JCODE_RUN_AUTO_POKE_MAX_TURNS", max_turns.to_string());
    }
    cmd.arg("run");
    if args.json {
        cmd.arg("--json");
    }
    if args.ndjson {
        cmd.arg("--ndjson");
    }
    cmd.arg(message);
    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("jcode run exited with status {}", status);
    }
    Ok(())
}

fn jcode_binary_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.parent()
                .map(|parent| parent.join(format!("jcode{}", std::env::consts::EXE_SUFFIX)))
        })
        .unwrap_or_else(|| PathBuf::from("jcode"))
}

fn run_skills(args: SkillsArgs) -> Result<()> {
    match args.command {
        SkillsCommand::List => jcode::cli::commands::run_skills_list_command(),
        SkillsCommand::Show { name } => jcode::cli::commands::run_skills_show_command(&name),
        SkillsCommand::Sync { force } => jcode::cli::commands::run_skills_sync_command(force),
        SkillsCommand::Doctor => jcode::cli::commands::run_skills_doctor_command(),
    }
}

async fn run_smoke(args: SmokeArgs) -> Result<()> {
    let workspace = if let Some(cwd) = args.cwd {
        PathBuf::from(cwd)
    } else {
        create_temp_workspace()?
    };

    std::fs::create_dir_all(&workspace)?;
    std::env::set_current_dir(&workspace)?;
    eprintln!("Harness workspace: {}", workspace.display());

    let provider: Arc<dyn Provider> = Arc::new(NoopProvider);
    let registry = Registry::new(provider).await;

    let session_id = new_id("harness");
    let base_ctx = ToolContext {
        session_id: session_id.clone(),
        message_id: session_id.clone(),
        tool_call_id: String::new(),
        working_dir: Some(workspace.clone()),
        stdin_request_tx: None,
        graceful_shutdown_signal: None,
        execution_mode: ToolExecutionMode::Direct,
    };

    let mut cases = vec![
        ToolCase {
            name: "write",
            label: "write sample.txt",
            input: json!({"file_path": "sample.txt", "content": "alpha\nbeta\n"}),
        },
        ToolCase {
            name: "read",
            label: "read sample.txt",
            input: json!({"file_path": "sample.txt"}),
        },
        ToolCase {
            name: "edit",
            label: "edit sample.txt (alpha -> alpha1)",
            input: json!({"file_path": "sample.txt", "old_string": "alpha", "new_string": "alpha1"}),
        },
        ToolCase {
            name: "multiedit",
            label: "multiedit sample.txt",
            input: json!({"file_path": "sample.txt", "edits": [{"old_string": "alpha1", "new_string": "alpha2"}, {"old_string": "beta", "new_string": "beta1"}]}),
        },
        ToolCase {
            name: "patch",
            label: "patch sample.txt",
            input: json!({"patch_text": "--- a/sample.txt\n+++ b/sample.txt\n@@ -1,2 +1,3 @@\n alpha2\n beta1\n+gamma\n"}),
        },
        ToolCase {
            name: "apply_patch",
            label: "apply_patch add file",
            input: json!({"patch_text": "*** Begin Patch\n*** Add File: added.txt\n+added\n*** End Patch\n"}),
        },
        ToolCase {
            name: "ls",
            label: "ls .",
            input: json!({"path": "."}),
        },
        ToolCase {
            name: "glob",
            label: "glob *.txt",
            input: json!({"pattern": "*.txt"}),
        },
        ToolCase {
            name: "grep",
            label: "grep gamma",
            input: json!({"pattern": "gamma", "path": "."}),
        },
        ToolCase {
            name: "bash",
            label: "bash pwd",
            input: json!({"command": "pwd"}),
        },
        ToolCase {
            name: "invalid",
            label: "invalid tool call",
            input: json!({"tool": "unknown", "error": "missing required field"}),
        },
        ToolCase {
            name: "todo",
            label: "todo write",
            input: json!({"todos": [{"content": "harness task", "status": "pending", "priority": "low", "id": "1"}]}),
        },
        ToolCase {
            name: "todo",
            label: "todo read",
            input: json!({}),
        },
        ToolCase {
            name: "batch",
            label: "batch ls + read",
            input: json!({"tool_calls": [{"tool": "ls", "parameters": {"path": "."}}, {"tool": "read", "parameters": {"file_path": "sample.txt"}}]}),
        },
    ];

    if args.include_network {
        cases.push(ToolCase {
            name: "webfetch",
            label: "webfetch example.com",
            input: json!({"url": "https://example.com", "format": "text"}),
        });
        cases.push(ToolCase {
            name: "websearch",
            label: "websearch rust async",
            input: json!({"query": "rust async await"}),
        });
        cases.push(ToolCase {
            name: "codesearch",
            label: "codesearch tokio spawn",
            input: json!({"query": "tokio::spawn"}),
        });
    }

    for (idx, case) in cases.iter().enumerate() {
        let ctx = ToolContext {
            tool_call_id: format!("harness-{}", idx + 1),
            ..base_ctx.clone()
        };
        println!("\n== {} ({}) ==", case.name, case.label);
        match registry.execute(case.name, case.input.clone(), ctx).await {
            Ok(output) => {
                if let Some(title) = output.title {
                    println!("[title] {}", title);
                }
                println!("{}", output.output);
            }
            Err(err) => println!("[error] {}", err),
        }
    }

    Ok(())
}

fn create_temp_workspace() -> Result<PathBuf> {
    let mut path = std::env::temp_dir();
    path.push(format!("jcode-harness-{}", new_id("run")));
    Ok(path)
}

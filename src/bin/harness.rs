use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use jcode::cli::provider_init::ProviderChoice;
use jcode::id::new_id;
use jcode::message::{Message, ToolDefinition};
use jcode::provider::{EventStream, Provider};
use jcode::tool::{Registry, ToolContext, ToolExecutionMode};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "jcode-harness")]
#[command(about = "Standalone jcode harness utilities. With no command, starts interactive jcode.")]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a project for the full jcode-harness experience
    Init(InitArgs),
    /// Run the deterministic tool harness smoke test
    Smoke(SmokeArgs),
    /// Run a single goal by delegating to the jcode run path with skill routing
    Run(RunArgs),
    /// Manage embedded and local skills
    Skills(SkillsArgs),
    /// Run Clean Code Guardian quality checks
    #[command(name = "clean-code")]
    CleanCode(CleanCodeArgs),
}

#[derive(Parser)]
struct InitArgs {
    /// Project directory to initialize (defaults to current directory)
    #[arg(long)]
    cwd: Option<String>,
    /// Overwrite existing generated files
    #[arg(long)]
    force: bool,
    /// Non-interactive defaults, leave questions in .jcode/INIT_QUESTIONS.md
    #[arg(long)]
    yes: bool,
    /// Skip creating .jcode/memory_wiki
    #[arg(long)]
    no_memory_wiki: bool,
    /// Emit JSON report
    #[arg(long)]
    json: bool,
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
struct CleanCodeArgs {
    #[command(subcommand)]
    command: CleanCodeCommand,
}

#[derive(Subcommand)]
enum CleanCodeCommand {
    /// Run the offline Clean Code Guardian quality gate
    Check {
        /// Project directory to use as root
        #[arg(long)]
        cwd: Option<String>,
        /// Files or directories to scan, defaults to cwd
        paths: Vec<PathBuf>,
        /// Emit JSON report
        #[arg(long)]
        json: bool,
        /// Exit non-zero when findings at this severity or higher are present
        #[arg(long, value_enum, default_value = "error")]
        fail_on: HarnessFailOn,
    },
    /// Print the built-in clean-code rule pack YAML
    Rules,
}

#[derive(Clone, ValueEnum)]
enum HarnessFailOn {
    Info,
    Warning,
    Error,
}

impl From<HarnessFailOn> for jcode::clean_code::Severity {
    fn from(value: HarnessFailOn) -> Self {
        match value {
            HarnessFailOn::Info => Self::Info,
            HarnessFailOn::Warning => Self::Warning,
            HarnessFailOn::Error => Self::Error,
        }
    }
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
    jcode::cli::terminal::install_panic_hook();

    let args = Args::parse();
    match args.command {
        None => jcode::run().await,
        Some(Command::Init(args)) => run_init(args),
        Some(Command::Smoke(args)) => run_smoke(args).await,
        Some(Command::Run(args)) => run_goal(args).await,
        Some(Command::Skills(args)) => run_skills(args),
        Some(Command::CleanCode(args)) => run_clean_code(args),
    }
}

fn run_init(args: InitArgs) -> Result<()> {
    let root = args
        .cwd
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
    let report = jcode::project_init::run_project_init(
        &root,
        jcode::project_init::ProjectInitOptions {
            force: args.force,
            yes: args.yes,
            include_memory_wiki: !args.no_memory_wiki,
        },
    )?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "Initialized jcode-harness project at {}",
            report.root.display()
        );
        println!(
            "Detected stack: {}",
            if report.detected_stack.is_empty() {
                "none".into()
            } else {
                report.detected_stack.join(", ")
            }
        );
        println!("Files written: {}", report.files_written.len());
        for path in &report.files_written {
            println!("  wrote {}", path.display());
        }
        if !report.files_skipped.is_empty() {
            println!(
                "Files skipped: {} (use --force to overwrite)",
                report.files_skipped.len()
            );
            for path in &report.files_skipped {
                println!("  skipped {}", path.display());
            }
        }
        println!("Next steps:");
        for step in &report.next_steps {
            println!("  - {}", step);
        }
    }
    Ok(())
}

async fn run_goal(args: RunArgs) -> Result<()> {
    if let Some(cwd) = &args.cwd {
        std::env::set_current_dir(cwd)?;
    }
    if let Some(profile_name) = args
        .provider_profile
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        jcode::provider_catalog::apply_named_provider_profile_env(profile_name)?;
        jcode::env::set_var("JCODE_PROVIDER_PROFILE_NAME", profile_name);
        jcode::env::set_var("JCODE_PROVIDER_PROFILE_ACTIVE", "1");
    }

    let provider_choice = if args.provider_profile.is_some() {
        ProviderChoice::OpenaiCompatible
    } else if let Some(provider) = args.provider.as_deref() {
        ProviderChoice::from_str(provider, true)
            .map_err(|err| anyhow::anyhow!("invalid provider '{}': {}", provider, err))?
    } else {
        ProviderChoice::Auto
    };
    let message =
        jcode::cli::commands::with_auto_skill_preface(&args.goal, &args.skill, args.skills.into());
    if args.dry_run {
        println!("{}", message);
        return Ok(());
    }

    let provider = if args.json || args.ndjson {
        jcode::cli::provider_init::init_provider_quiet(&provider_choice, args.model.as_deref())
            .await?
    } else {
        jcode::cli::provider_init::init_provider_for_validation(
            &provider_choice,
            args.model.as_deref(),
        )
        .await?
    };
    let registry = Registry::new(provider.clone()).await;
    let mut agent = jcode::agent::Agent::new(provider.clone(), registry);

    if args.ndjson {
        println!(
            "{}",
            serde_json::to_string(&json!({
                "type": "start",
                "session_id": agent.session_id(),
                "provider": provider.name(),
                "model": provider.model(),
            }))?
        );
    }

    let max_turns = args.max_turns.unwrap_or(1).max(1);
    let mut text = String::new();
    for turn in 0..max_turns {
        let prompt = if turn == 0 {
            message.as_str()
        } else {
            "Continue if needed, otherwise summarize completion."
        };
        if args.json || args.ndjson {
            let output = agent.run_once_capture(prompt).await?;
            if !text.is_empty() {
                text.push_str("\n\n");
            }
            text.push_str(&output);
        } else {
            agent.run_once(prompt).await?;
        }
    }

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "session_id": agent.session_id(),
                "provider": provider.name(),
                "model": provider.model(),
                "text": text,
                "usage": agent.last_usage(),
            }))?
        );
    } else if args.ndjson {
        println!(
            "{}",
            serde_json::to_string(&json!({
                "type": "done",
                "session_id": agent.session_id(),
                "text": text,
                "usage": agent.last_usage(),
            }))?
        );
    }
    Ok(())
}

fn run_skills(args: SkillsArgs) -> Result<()> {
    match args.command {
        SkillsCommand::List => jcode::cli::commands::run_skills_list_command(),
        SkillsCommand::Show { name } => jcode::cli::commands::run_skills_show_command(&name),
        SkillsCommand::Sync { force } => jcode::cli::commands::run_skills_sync_command(force),
        SkillsCommand::Doctor => jcode::cli::commands::run_skills_doctor_command(),
    }
}

fn run_clean_code(args: CleanCodeArgs) -> Result<()> {
    match args.command {
        CleanCodeCommand::Rules => jcode::cli::commands::run_clean_code_rules_command(),
        CleanCodeCommand::Check {
            cwd,
            paths,
            json,
            fail_on,
        } => {
            if let Some(cwd) = cwd {
                std::env::set_current_dir(cwd)?;
            }
            jcode::cli::commands::run_clean_code_check_command(paths, json, fail_on.into())
        }
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

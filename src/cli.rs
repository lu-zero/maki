use std::path::PathBuf;

use color_eyre::Result;
use color_eyre::eyre::bail;
use winnow_args::{Args, Subcommand, ValueEnum};

use maki_agent::tools::{all_builtin_tool_names, is_builtin_tool};

use crate::print::OutputFormat;

#[derive(Clone, Default, ValueEnum)]
pub enum PromptVariant {
    #[default]
    System,
    Research,
    General,
}

#[derive(Clone, Default, ValueEnum)]
pub enum InputFormat {
    #[default]
    Text,
    StreamJson,
}

// Only one way to name the session to load, or the resolver would quietly pick
// one. `--fork-session` requires it, since forking nothing used to start a
// blank session.
#[derive(Args)]
#[arg(
    name = "maki",
    version,
    about = "AI coding agent for the terminal",
    group("loaded")
)]
pub struct Cli {
    #[arg(subcommand)]
    pub command: Option<Command>,

    /// Non-interactive mode. Runs the prompt and exits. Compatible with Claude Code's --print flag
    #[arg(short, long)]
    pub print: bool,

    /// Attach an image to the prompt in --print mode as vision content (repeatable)
    #[arg(long = "image", value_name = "PATH")]
    pub images: Vec<PathBuf>,

    /// Model spec (provider/model-id). Defaults to last used model, or claude-opus-4-6
    #[arg(short, long)]
    pub model: Option<String>,

    /// Include full turn-by-turn messages in --print output
    #[arg(long)]
    pub verbose: bool,

    /// Resume the most recent session in this directory
    #[arg(short = 'c', long = "continue", group = "loaded")]
    pub continue_session: bool,

    /// Resume a specific session by its ID
    #[arg(
        short = 'r',
        short = 's',
        long,
        visible_alias = "session",
        group = "loaded"
    )]
    pub resume: Option<String>,

    /// Output format for --print mode
    #[arg(long, default = "text")]
    pub output_format: OutputFormat,

    /// Input format (text or stream-json for SDK mode)
    #[arg(long, default = "text")]
    pub input_format: InputFormat,

    /// Skip loading custom commands from .maki/commands, .claude/commands, etc.
    #[arg(long)]
    pub no_commands: bool,

    /// Skip user `init.lua` files (global and project) but keep the Lua
    /// host and every builtin plugin running, so tools and the default
    /// keymap still load. Use this to recover from a broken `init.lua`
    /// or keymap override. Only Lua `init.lua` files are affected;
    /// `permissions.toml`, custom commands, and env files load as usual.
    #[arg(long)]
    pub no_plugins: bool,

    /// Run plugin Lua on the interpreter with full debug info (no native codegen)
    #[arg(long)]
    pub no_jit: bool,

    /// Skip all permission prompts (allow everything)
    #[arg(long, alias = "dangerously-skip-permissions")]
    pub yolo: bool,

    /// Load shared project `.maki` config for this run without asking and
    /// without recording an answer. For containers and CI, where the state
    /// directory is thrown away anyway. Only use it on a project you trust.
    #[arg(long)]
    pub trust: bool,

    /// Exit after the agent completes (for automation workflows)
    #[arg(long)]
    pub exit_on_done: bool,

    /// Pre-approve tools (comma-separated). Accepts PascalCase (Claude Code) or snake_case.
    #[arg(long, delimiter = ',', visible_alias = "allowedTools")]
    pub allowed_tools: Vec<String>,

    /// Disallowed tools (comma-separated).
    #[arg(long, delimiter = ',', visible_alias = "disallowedTools")]
    pub disallowed_tools: Vec<String>,

    /// Write this run under a chosen session ID, unless one already exists there
    #[arg(long)]
    pub session_id: Option<String>,

    /// Fork the loaded session under a new ID
    #[arg(
        long,
        requires(
            "loaded",
            message = "`--fork-session` requires `--continue` or `--resume`"
        )
    )]
    pub fork_session: bool,

    /// Maximum number of agent turns
    #[arg(long)]
    pub max_turns: Option<u32>,

    /// System prompt override
    #[arg(long)]
    pub system_prompt: Option<String>,

    /// Append to system prompt
    #[arg(long)]
    pub append_system_prompt: Option<String>,

    /// Permission mode for SDK
    #[arg(long)]
    pub permission_mode: Option<String>,

    /// Include partial streaming messages in SDK output
    #[arg(long)]
    pub include_partial_messages: bool,

    /// Permission prompt tool (accepted for compat, used in SDK mode)
    #[arg(long, hide)]
    pub permission_prompt_tool: Option<String>,

    // Accepted but ignored, so Claude Code SDK callers don't break.
    #[arg(long, hide)]
    pub fallback_model: Option<String>,
    #[arg(long, hide)]
    pub settings: Option<String>,
    #[arg(long, hide)]
    pub setting_sources: Option<String>,
    #[arg(long, hide)]
    pub add_dir: Option<String>,
    #[arg(long, hide)]
    pub strict_mcp_config: bool,
    #[arg(long, hide)]
    pub include_hook_events: bool,
    #[arg(long, hide)]
    pub mcp_config: Option<String>,
    #[arg(long, hide)]
    pub tools: Option<String>,
    #[arg(long, hide)]
    pub betas: Option<String>,
    #[arg(long, hide)]
    pub max_thinking_tokens: Option<String>,
    #[arg(long, hide)]
    pub effort: Option<String>,
    #[arg(long, hide)]
    pub json_schema: Option<String>,
    #[arg(long, hide)]
    pub max_budget_usd: Option<String>,
    #[arg(long, hide)]
    pub thinking: Option<String>,
    #[arg(long, hide)]
    pub thinking_display: Option<String>,

    /// Initial prompt (reads stdin if piped)
    #[arg(positional, value_name = "PROMPT")]
    pub initial_prompt: Option<String>,
}

impl Cli {
    pub fn warn_ignored_flags(&self) {
        let ignored = [
            ("fallback-model", self.fallback_model.is_some()),
            ("settings", self.settings.is_some()),
            ("setting-sources", self.setting_sources.is_some()),
            ("add-dir", self.add_dir.is_some()),
            ("strict-mcp-config", self.strict_mcp_config),
            ("include-hook-events", self.include_hook_events),
            ("mcp-config", self.mcp_config.is_some()),
            ("tools", self.tools.is_some()),
            ("betas", self.betas.is_some()),
            ("max-thinking-tokens", self.max_thinking_tokens.is_some()),
            ("effort", self.effort.is_some()),
            ("json-schema", self.json_schema.is_some()),
            ("max-budget-usd", self.max_budget_usd.is_some()),
            ("thinking", self.thinking.is_some()),
            ("thinking-display", self.thinking_display.is_some()),
        ];
        for (flag, set) in &ignored {
            if *set {
                eprintln!("warning: --{flag} is accepted but ignored");
            }
        }
    }

    pub fn is_sdk_mode(&self) -> bool {
        self.print && matches!(self.input_format, InputFormat::StreamJson)
    }
}

#[derive(Subcommand)]
pub enum Command {
    /// Manage API authentication
    Auth {
        #[arg(subcommand)]
        action: AuthAction,
    },
    /// List all available models
    Models {
        /// Refetch the models.dev catalog, ignoring its 24h cache
        #[arg(long)]
        refresh: bool,
    },
    /// Manage sessions
    Session {
        #[arg(subcommand)]
        action: SessionAction,
    },
    /// Run the index tool on a file to see how it looks like
    Index {
        #[arg(positional)]
        path: String,
    },
    /// Manage MCP server authentication
    Mcp {
        #[arg(subcommand)]
        action: McpAction,
    },
    /// Update maki to the latest version
    Update {
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
        /// Disable syntax highlighting
        #[arg(long)]
        no_color: bool,
    },
    /// Rollback to the previous version
    Rollback,
    /// Run as an ACP (Agent Client Protocol) server over stdio
    Acp {
        /// Model spec (provider/model-id)
        #[arg(short, long)]
        model: Option<String>,
        /// Skip all permission prompts
        #[arg(long)]
        yolo: bool,
    },
    /// Show the rendered system prompt or tool definitions
    Prompt {
        /// Prompt variant: system (default), research, general
        #[arg(positional, default = "system")]
        variant: PromptVariant,
        /// Append the plan mode reminder to the system prompt
        #[arg(long)]
        plan: bool,
        /// Show tool definitions (JSON) instead of prompt text
        #[arg(long)]
        tools: bool,
        /// With --tools: show only tool names, one per line
        #[arg(long, requires = "tools")]
        names: bool,
    },
    /// Data migration utilities
    Migrate {
        #[arg(subcommand)]
        action: MigrateAction,
    },
    /// Manage projects that may load automatic shared project configuration
    Trust {
        #[arg(subcommand)]
        action: TrustAction,
    },
}

#[derive(Subcommand)]
pub enum TrustAction {
    /// Trust a project to load automatic shared .maki configuration
    Add {
        /// Project to trust. Defaults to the current directory
        #[arg(positional)]
        path: Option<PathBuf>,
        /// Skip the confirmation prompt
        #[arg(long)]
        yes: bool,
    },
    /// Remove a stored project trust decision
    Remove {
        /// Project to remove. Defaults to the current directory
        #[arg(positional)]
        path: Option<PathBuf>,
    },
    /// List stored project trust decisions
    List,
}

#[derive(Subcommand)]
pub enum SessionAction {
    /// List sessions
    List {
        /// Show sessions from all projects
        #[arg(short, long)]
        global: bool,
        /// Print machine-readable session summaries, one JSON array
        #[arg(long)]
        json: bool,
    },
    /// Delete a session
    Delete {
        /// Session ID (see `maki session list`)
        #[arg(positional, value_name = "SESSION_ID")]
        session_id: String,
        /// Skip the confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
    /// Write a session as a portable JSON transcript to stdout
    Export {
        /// Session ID (see `maki session list`)
        #[arg(value_name = "SESSION_ID")]
        session_id: String,
    },
    /// Store a JSON transcript as a new session in this directory and print its ID
    Import {
        /// Transcript file, or stdin when omitted or `-`
        #[arg(value_name = "FILE")]
        file: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum MigrateAction {
    /// Migrate files from ~/.maki/ to XDG directories
    Xdg,
    /// Print a prompt that ports your old provider scripts to Lua plugins
    Providers,
}

#[derive(Subcommand)]
pub enum McpAction {
    /// Authenticate with an MCP server
    Auth {
        /// Server name from config
        #[arg(positional)]
        server: String,
    },
    /// Remove stored OAuth credentials for an MCP server
    Logout {
        /// Server name from config
        #[arg(positional)]
        server: String,
    },
}

#[derive(Subcommand)]
pub enum AuthAction {
    /// Authenticate with a provider (interactive if no provider specified)
    Login {
        /// Provider slug (e.g. zai, openai, xai). Omit for interactive selection.
        #[arg(positional)]
        provider: Option<String>,
    },
    /// Remove stored credentials for a provider
    Logout {
        /// Provider slug (e.g. openai, xai)
        #[arg(positional)]
        provider: String,
    },
    /// Show authentication status for all providers
    Status,
}

pub fn normalize_tool_name(name: &str) -> Result<String> {
    let mut result = String::with_capacity(name.len() + 4);
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    if !is_builtin_tool(&result) {
        bail!(
            "unknown tool '{}'. Valid tools: {}",
            name,
            all_builtin_tool_names().join(", ")
        );
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;
    use winnow_args::Args;

    const SESSION_ID: &str = "01965087-4c71-7f00-8000-000000000000";
    // The flag's `message` repeats this. The attribute takes a literal.
    const FORK_NEEDS_SESSION: &str = "`--fork-session` requires `--continue` or `--resume`";

    #[test_case("Read", "read")]
    #[test_case("Bash", "bash")]
    #[test_case("CodeExecution", "code_execution")]
    #[test_case("code_execution", "code_execution"; "snake_passthrough")]
    fn normalize_tool_name_valid_inputs(input: &str, expected: &str) {
        assert_eq!(normalize_tool_name(input).unwrap(), expected);
    }

    #[test]
    fn normalize_tool_name_rejects_unknown() {
        let result = normalize_tool_name("NonExistentTool");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown tool"));
    }

    #[test]
    fn normalize_tool_name_multi_edit_rejects_snake_variant() {
        assert!(normalize_tool_name("MultiEdit").is_err());
    }

    /// `--session-id` with `-c` stays legal: continue the latest, but write
    /// under this id.
    #[test_case(&["-c", "-r", SESSION_ID], false ; "two sessions to load")]
    #[test_case(&["-c", "-s", SESSION_ID], false ; "two sessions to load through the short alias")]
    #[test_case(&["-c", "--session", SESSION_ID], false ; "two sessions to load through the long alias")]
    #[test_case(&["--fork-session"], false ; "a fork with nothing to fork")]
    #[test_case(&["--fork-session", "-c"], true ; "a fork of the latest")]
    #[test_case(&["--fork-session", "-r", SESSION_ID], true ; "a fork of a named session")]
    #[test_case(&["-c", "--session-id", SESSION_ID], true ; "a redirected continue")]
    fn session_flag_combinations(args: &[&str], accepted: bool) {
        let argv = std::iter::once("maki").chain(args.iter().copied());

        assert_eq!(Cli::parse_from_argv(argv).is_ok(), accepted);
    }

    #[test]
    fn fork_without_a_session_says_why() {
        let Err(error) = Cli::parse_from(["--fork-session"]) else {
            panic!("fork with nothing to fork was accepted");
        };
        assert_eq!(error.kind(), winnow_args::ErrorKind::MissingOneOf);
        assert_eq!(error.to_string(), FORK_NEEDS_SESSION);
    }

    #[test]
    fn visible_aliases_are_listed() {
        let Err(error) = Cli::parse_from(["-h"]) else {
            panic!("help was a successful parse");
        };
        let text = error.render_help("maki").expect("help text");
        assert!(text.contains("-r, -s, --resume, --session"), "{text}");
        assert!(text.contains("--allowed-tools, --allowedTools"), "{text}");
        assert!(
            text.contains("--disallowed-tools, --disallowedTools"),
            "{text}"
        );
        assert!(!text.contains("dangerously-skip-permissions"), "{text}");
    }

    #[test]
    fn compat_spellings_prompt_and_subcommand() {
        let yolo = Cli::parse_from(["--dangerously-skip-permissions"]).unwrap();
        assert!(yolo.yolo);

        let tools = Cli::parse_from([
            "--allowedTools",
            "read,bash",
            "--disallowedTools",
            "web_search",
        ])
        .unwrap();
        assert_eq!(tools.allowed_tools, ["read", "bash"]);
        assert_eq!(tools.disallowed_tools, ["web_search"]);

        let resumed = Cli::parse_from(["--session", SESSION_ID]).unwrap();
        assert_eq!(resumed.resume.as_deref(), Some(SESSION_ID));

        let prompt = Cli::parse_from(["fix the parser"]).unwrap();
        assert_eq!(prompt.initial_prompt.as_deref(), Some("fix the parser"));
        assert!(prompt.command.is_none());

        let auth = Cli::parse_from(["auth", "status"]).unwrap();
        assert!(matches!(
            auth.command,
            Some(Command::Auth {
                action: AuthAction::Status
            })
        ));

        let format = Cli::parse_from([
            "--print",
            "--input-format",
            "stream-json",
            "--output-format",
            "json",
        ])
        .unwrap();
        assert!(matches!(format.input_format, InputFormat::StreamJson));
        assert!(matches!(format.output_format, OutputFormat::Json));
        assert!(format.is_sdk_mode());

        assert!(Cli::parse_from(["--no-plugins", "session", "list"]).is_ok());
        assert!(Cli::parse_from(["session", "list", "--no-plugins"]).is_err());
    }

    #[test]
    fn prompt_names_need_tools_and_the_variant_defaults() {
        let bare = Cli::parse_from(["prompt"]).unwrap();
        assert!(matches!(
            bare.command,
            Some(Command::Prompt {
                variant: PromptVariant::System,
                plan: false,
                tools: false,
                names: false,
            })
        ));

        assert!(Cli::parse_from(["prompt", "--names"]).is_err());

        let named = Cli::parse_from(["prompt", "research", "--tools", "--names"]).unwrap();
        assert!(matches!(
            named.command,
            Some(Command::Prompt {
                variant: PromptVariant::Research,
                tools: true,
                names: true,
                ..
            })
        ));
    }
}

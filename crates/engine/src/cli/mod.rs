use std::sync::Arc;
pub use tokio::sync::broadcast::error::RecvError;
use sqlx::SqlitePool;
use rustyline::{DefaultEditor, error::ReadlineError};

use crate::workspace::Workspace;
use crate::llm::LlmClient;

// ─── ANSI Color Constants ──────────────────────────────────────────

const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

// ─── CLI Console ────────────────────────────────────────────────────

pub struct CliConsole {
    pub workspace: Arc<Workspace>,
    pub pool: SqlitePool,
    pub llm_client: Arc<LlmClient>,
}

impl CliConsole {
    pub fn new(
        workspace: Arc<Workspace>,
        pool: SqlitePool,
        llm_client: Arc<LlmClient>,
    ) -> Self {
        Self { workspace, pool, llm_client }
    }

    pub async fn run(&self) {
        println!("{}", format!(
            "\n{}╔══════════════════════════════════════════╗{}\n\
               {}║   AI Trading Assistant — CLI Console    ║{}\n\
               {}║   Type 'help' for available commands   ║{}\n\
               {}╚══════════════════════════════════════════╝{}\n",
            CYAN, RESET, CYAN, RESET, CYAN, RESET, CYAN, RESET
        ));

        let mut rl = match DefaultEditor::new() {
            Ok(editor) => editor,
            Err(_) => {
                eprintln!("Failed to initialize readline, falling back to basic input");
                self.run_basic().await;
                return;
            }
        };

        loop {
            let prompt = format!("{}🦀 >{} ", GREEN, RESET);
            match rl.readline(&prompt) {
                Ok(line) => {
                    let line = line.trim().to_string();
                    if line.is_empty() { continue; }
                    let _ = rl.add_history_entry(&line);
                    if !self.execute(&line).await {
                        break;
                    }
                }
                Err(ReadlineError::Interrupted) => continue,
                Err(ReadlineError::Eof) => break,
                Err(err) => {
                    eprintln!("Input error: {}", err);
                    break;
                }
            }
        }
    }

    async fn run_basic(&self) {
        use tokio::io::{AsyncBufReadExt, BufReader};
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut buf = String::new();

        loop {
            print!("{}🦀 >{} ", GREEN, RESET);
            buf.clear();
            match reader.read_line(&mut buf).await {
                Ok(0) => break,
                Ok(_) => {
                    let line = buf.trim().to_string();
                    if line.is_empty() { continue; }
                    if !self.execute(&line).await { break; }
                }
                Err(_) => break,
            }
        }
    }

    /// Execute a command. Returns false to exit.
    async fn execute(&self, input: &str) -> bool {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() { return true; }

        let cmd = parts[0].to_lowercase();
        let args: Vec<&str> = parts[1..].to_vec();

        match cmd.as_str() {
            "help" | "?" => self.cmd_help(),
            "quit" | "exit" | "q" => {
                println!("{}Shutting down...{}", YELLOW, RESET);
                let _ = self.workspace.quit_session().await;
                return false;
            }
            "add" => self.cmd_add(&args).await,
            "pause" => self.cmd_pause(&args).await,
            "stop" => self.cmd_stop(&args).await,
            "delete" => self.cmd_delete(&args).await,
            "list" | "ls" => self.cmd_list().await,
            "show" => self.cmd_show(&args).await,
            "dashboard" | "dash" => self.cmd_dashboard().await,
            "status" | "stat" => self.cmd_status().await,
            "config" => self.cmd_config(&args).await,
            "safety" => self.cmd_safety(&args).await,
            "manual" => self.cmd_manual(&args).await,
            "chat" => self.cmd_chat(&args).await,
            "watch" => self.cmd_watch(&args).await,
            other => {
                println!("{}{}Unknown command: '{}'. Type 'help' for available commands.{}",
                    RED, BOLD, other, RESET);
            }
        }
        true
    }

}

mod commands;

pub(crate) fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}

pub(crate) fn opt_decimal(v: &Option<rust_decimal::Decimal>) -> String {
    v.as_ref().map(|d| d.to_string()).unwrap_or_else(|| "—".to_string())
}

pub(crate) fn opt_decimal_short(v: &Option<rust_decimal::Decimal>) -> String {
    v.as_ref().map(|d| format!("{:.4}", d)).unwrap_or_else(|| "—".to_string())
}

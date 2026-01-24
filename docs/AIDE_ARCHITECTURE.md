# AIDE: AI Development Environment

**Design Document v0.1**
**Date:** 2026-01-20
**Status:** Draft

## Overview

AIDE (AI Development Environment) is an intelligent coding agent optimized for Rush shell. It combines Claude's reasoning with Rush's persistent sessions, job control, and performance monitoring to create an AI-native development platform.

### Core Principles (Learned from Helix)

1. **LLMs at edges, machines in middle** - Use Claude for generation/translation, deterministic logic for decisions
2. **State in files, not memory** - Everything persists to `.aide/session-X/` for resumability
3. **Strange Loop learning** - Failures become guardrails, learned exactly once
4. **Deterministic risk gating** - Parse-based safety analysis, not LLM judgment
5. **Token budget enforcement** - Hard limits prevent context overflow
6. **Rush as source of truth** - Session state lives in Rush daemon, not agent memory

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    User Interface                        │
│                   (REPL or TUI)                          │
└────────────────────┬────────────────────────────────────┘
                     │
                     ↓
┌─────────────────────────────────────────────────────────┐
│                   AIDE Agent Loop                        │
├─────────────────────────────────────────────────────────┤
│  1. Check token budget                                   │
│  2. Load session state (from Rush + artifacts)          │
│  3. Build agent context (selective, condensed)          │
│  4. Call Claude with tools                              │
│  5. Execute tools with risk gating                      │
│  6. Learn from failures (Strange Loop)                  │
│  7. Persist state updates                               │
└────────────────────┬────────────────────────────────────┘
                     │
                     ↓
┌─────────────────────────────────────────────────────────┐
│              Rush Session (Persistent)                   │
├─────────────────────────────────────────────────────────┤
│  • Session ID, environment, working directory           │
│  • Command history (compact)                            │
│  • Running jobs + status                                │
│  • Performance stats (structured)                       │
│  • Exit codes, timestamps                               │
└─────────────────────────────────────────────────────────┘
                     │
                     ↓
┌─────────────────────────────────────────────────────────┐
│           Session Artifacts (.aide/session-X/)          │
├─────────────────────────────────────────────────────────┤
│  state.yaml      - Session metadata                     │
│  summary.md      - Human-readable accomplishments       │
│  guardrails.md   - Learned constraints from failures    │
│  perf.jsonl      - Performance data (structured)        │
│  commands.log    - Command history (compact)            │
│  decisions.md    - Design choices made                  │
│  context.yaml    - Token budget usage tracking          │
└─────────────────────────────────────────────────────────┘
```

## Agent Loop: Detailed Design

### Core Data Structures

```rust
/// AIDE agent instance managing a development session
pub struct AIDE {
    /// Unique session identifier
    session_id: u64,

    /// Rush daemon client for session management
    rush_client: RushDaemonClient,

    /// Claude API client
    claude_client: Anthropic,

    /// Current conversation context
    context: AgentContext,

    /// Token budget tracker
    budget: TokenBudget,

    /// Learned guardrails from failures
    guardrails: Vec<Guardrail>,

    /// Session artifact directory
    state_dir: PathBuf,  // .aide/session-X/

    /// Message history (working memory)
    messages: Vec<Message>,

    /// Performance baseline for anomaly detection
    perf_baseline: PerfBaseline,
}

/// Agent working context
pub struct AgentContext {
    /// System prompt (Rush expertise)
    system_prompt: String,

    /// Session summary (condensed history)
    session_summary: String,

    /// Active files being edited
    open_files: HashMap<PathBuf, String>,

    /// Current focus area
    focus: Option<String>,

    /// Token counts per section
    token_usage: TokenUsage,
}

/// Token budget allocation
pub struct TokenBudget {
    system_prompt: usize,        // 8,000 tokens
    session_summary: usize,      // 5,000 tokens
    conversation: usize,         // 150,000 tokens
    code_context: usize,         // 30,000 tokens
    headroom: usize,             // 7,000 tokens
    // Total: 200,000 tokens
}

impl TokenBudget {
    pub const fn default() -> Self {
        Self {
            system_prompt: 8_000,
            session_summary: 5_000,
            conversation: 150_000,
            code_context: 30_000,
            headroom: 7_000,
        }
    }

    pub fn total(&self) -> usize {
        self.system_prompt + self.session_summary +
        self.conversation + self.code_context + self.headroom
    }
}

/// Learned constraint from failure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guardrail {
    /// When this was learned
    timestamp: DateTime<Utc>,

    /// What we tried to do
    attempted_action: String,

    /// What went wrong
    error_message: String,

    /// The constraint to remember
    rule: String,

    /// Context for pattern matching
    context_tags: Vec<String>,
}
```

### Main Agent Loop

```rust
impl AIDE {
    /// Main execution loop - process one user message
    pub async fn execute(&mut self, user_msg: &str) -> Result<String> {
        // ═══════════════════════════════════════════════════════
        // Step 1: Check token budget
        // ═══════════════════════════════════════════════════════
        self.enforce_budget().await?;

        // ═══════════════════════════════════════════════════════
        // Step 2: Load session state from Rush + artifacts
        // ═══════════════════════════════════════════════════════
        let session_state = self.load_session_state().await?;

        // ═══════════════════════════════════════════════════════
        // Step 3: Build agent context (selective loading)
        // ═══════════════════════════════════════════════════════
        self.update_context(session_state)?;

        // ═══════════════════════════════════════════════════════
        // Step 4: Add user message to conversation
        // ═══════════════════════════════════════════════════════
        self.messages.push(Message {
            role: "user".to_string(),
            content: user_msg.to_string(),
        });

        // ═══════════════════════════════════════════════════════
        // Step 5: Call Claude with tools
        // ═══════════════════════════════════════════════════════
        let response = self.call_claude().await?;

        // ═══════════════════════════════════════════════════════
        // Step 6: Process response (tool calls or text)
        // ═══════════════════════════════════════════════════════
        let mut final_response = String::new();

        loop {
            match response.stop_reason.as_str() {
                "end_turn" => {
                    // Claude is done, extract text response
                    final_response = self.extract_text_content(&response);
                    break;
                }

                "tool_use" => {
                    // Execute tools with risk gating
                    let tool_results = self.execute_tools(&response).await?;

                    // Add tool results to conversation
                    self.messages.push(Message {
                        role: "user".to_string(),
                        content: serde_json::to_string(&tool_results)?,
                    });

                    // Continue conversation
                    response = self.call_claude().await?;
                }

                _ => {
                    bail!("Unexpected stop reason: {}", response.stop_reason);
                }
            }
        }

        // ═══════════════════════════════════════════════════════
        // Step 7: Persist state updates
        // ═══════════════════════════════════════════════════════
        self.save_state().await?;

        Ok(final_response)
    }

    /// Enforce token budget - compress if needed
    async fn enforce_budget(&mut self) -> Result<()> {
        let current_tokens = self.estimate_tokens();
        let budget_total = self.budget.total();

        if current_tokens > budget_total {
            // Need to compress conversation
            let overflow = current_tokens - budget_total;

            eprintln!("⚠️  Token budget exceeded by {}", overflow);
            eprintln!("   Compressing conversation history...");

            // Summarize old messages using Claude
            let summary = self.summarize_old_messages().await?;

            // Remove old messages, keep summary
            let cutoff = self.messages.len() / 3;  // Keep recent 2/3
            self.messages.drain(0..cutoff);

            // Prepend summary
            self.messages.insert(0, Message {
                role: "system".to_string(),
                content: format!("## Previous Session Summary\n\n{}", summary),
            });

            // Append summary to session artifacts
            self.append_summary(&summary).await?;

            eprintln!("✓  Compression complete");
        }

        Ok(())
    }

    /// Load current session state from Rush daemon + artifacts
    async fn load_session_state(&self) -> Result<SessionState> {
        // Query Rush daemon for live state
        let rush_state = self.rush_client.query_session(self.session_id).await?;

        // Load artifacts from disk
        let summary = fs::read_to_string(
            self.state_dir.join("summary.md")
        ).unwrap_or_default();

        let guardrails = self.load_guardrails()?;

        let perf_data = self.load_perf_data()?;

        Ok(SessionState {
            rush_state,
            summary,
            guardrails,
            perf_data,
        })
    }

    /// Update agent context with session state
    fn update_context(&mut self, state: SessionState) -> Result<()> {
        // Update session summary (condensed)
        self.context.session_summary = self.condense_summary(&state.summary);

        // Update guardrails
        self.guardrails = state.guardrails;

        // Update performance baseline
        self.perf_baseline.update(&state.perf_data);

        // Detect anomalies
        if let Some(anomaly) = self.detect_perf_anomaly(&state.perf_data) {
            self.context.focus = Some(format!(
                "Performance anomaly detected: {}", anomaly
            ));
        }

        Ok(())
    }

    /// Call Claude API with current context
    async fn call_claude(&self) -> Result<Response> {
        let response = self.claude_client.messages.create(
            MessagesRequest {
                model: "claude-sonnet-4.5".to_string(),
                max_tokens: 8192,
                system: self.build_system_prompt(),
                messages: self.messages.clone(),
                tools: self.get_tools(),
            }
        ).await?;

        Ok(response)
    }

    /// Execute tool calls with risk gating
    async fn execute_tools(&mut self, response: &Response) -> Result<Vec<ToolResult>> {
        let mut results = Vec::new();

        for block in &response.content {
            if let ContentBlock::ToolUse { id, name, input } = block {
                // ═══════════════════════════════════════════════════
                // DETERMINISTIC RISK GATING
                // ═══════════════════════════════════════════════════
                let risk = self.compute_risk(name, input)?;

                if risk.requires_approval() {
                    self.ask_user_approval(name, input)?;
                }

                // ═══════════════════════════════════════════════════
                // CHECK GUARDRAILS
                // ═══════════════════════════════════════════════════
                if let Some(violation) = self.check_guardrails(name, input) {
                    eprintln!("⚠️  Guardrail violation: {}", violation.rule);

                    results.push(ToolResult {
                        tool_use_id: id.clone(),
                        content: format!("Error: {}", violation.rule),
                        is_error: true,
                    });

                    continue;
                }

                // ═══════════════════════════════════════════════════
                // EXECUTE TOOL
                // ═══════════════════════════════════════════════════
                match self.execute_tool(name, input).await {
                    Ok(result) => {
                        results.push(ToolResult {
                            tool_use_id: id.clone(),
                            content: result,
                            is_error: false,
                        });
                    }

                    Err(e) => {
                        eprintln!("❌ Tool execution failed: {}", e);

                        // ═══════════════════════════════════════════════
                        // STRANGE LOOP: Learn from failure
                        // ═══════════════════════════════════════════════
                        self.learn_guardrail(name, input, &e).await?;

                        results.push(ToolResult {
                            tool_use_id: id.clone(),
                            content: format!("Error: {}", e),
                            is_error: true,
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    /// Learn guardrail from tool execution failure (Strange Loop)
    async fn learn_guardrail(
        &mut self,
        tool_name: &str,
        tool_input: &Value,
        error: &Error
    ) -> Result<()> {
        eprintln!("🧠 Learning from failure...");

        // Create guardrail
        let guardrail = Guardrail {
            timestamp: Utc::now(),
            attempted_action: format!("{}({})", tool_name, tool_input),
            error_message: error.to_string(),
            rule: self.generate_guardrail_rule(tool_name, tool_input, error).await?,
            context_tags: self.extract_context_tags(tool_input),
        };

        // Add to memory
        self.guardrails.push(guardrail.clone());

        // Persist to disk
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.state_dir.join("guardrails.md"))?;

        writeln!(file, "\n## {}", guardrail.timestamp.format("%Y-%m-%d %H:%M:%S"))?;
        writeln!(file, "**Attempted:** {}", guardrail.attempted_action)?;
        writeln!(file, "**Error:** {}", guardrail.error_message)?;
        writeln!(file, "**Guardrail:** {}", guardrail.rule)?;

        eprintln!("✓  Guardrail learned and persisted");

        Ok(())
    }

    /// Check if tool call violates any learned guardrails
    fn check_guardrails(&self, tool_name: &str, input: &Value) -> Option<&Guardrail> {
        for guardrail in &self.guardrails {
            if self.matches_guardrail(tool_name, input, guardrail) {
                return Some(guardrail);
            }
        }
        None
    }

    /// Compute risk level deterministically (no LLM)
    fn compute_risk(&self, tool_name: &str, input: &Value) -> Result<RiskLevel> {
        let mut score = 0;

        match tool_name {
            // High-risk tools
            "execute_in_session" => {
                let cmd = input["command"].as_str().unwrap_or("");

                // Parse command with Rush
                if let Ok(ast) = rush::parser::parse(cmd) {
                    if ast.contains_destructive_ops() {
                        score += 30;  // rm -rf, etc.
                    }
                    if ast.modifies_system_paths() {
                        score += 20;  // /etc, /usr, etc.
                    }
                    if ast.has_network_access() {
                        score += 10;  // curl, wget, etc.
                    }
                    if ast.spawns_background_jobs() {
                        score += 5;
                    }
                    if ast.is_read_only() {
                        score -= 10;  // Bonus for safe ops
                    }
                } else {
                    score += 5;  // Parse failed = slightly risky
                }
            }

            "write_file" | "edit_file" => {
                let path = input["path"].as_str().unwrap_or("");

                if path.starts_with("/etc") || path.starts_with("/usr") {
                    score += 25;
                } else if path.contains("src/") {
                    score += 0;  // Expected
                } else {
                    score += 5;
                }
            }

            "spawn_job" => {
                score += 10;  // Background jobs = moderate risk
            }

            _ => {
                score += 0;  // Unknown tool, default safe
            }
        }

        Ok(RiskLevel::from_score(score))
    }

    /// Persist current state to disk
    async fn save_state(&self) -> Result<()> {
        // Save state metadata
        let state = SessionStateFile {
            session_id: self.session_id,
            created_at: self.context.created_at,
            last_active: Utc::now(),
            message_count: self.messages.len(),
            token_usage: self.context.token_usage.clone(),
        };

        let state_file = self.state_dir.join("state.yaml");
        fs::write(state_file, serde_yaml::to_string(&state)?)?;

        Ok(())
    }
}
```

## Risk Levels

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,        // 0-5 points
    Medium,     // 6-15 points
    High,       // 16-30 points
    Critical,   // 30+ points
}

impl RiskLevel {
    pub fn from_score(score: i32) -> Self {
        match score {
            0..=5 => RiskLevel::Low,
            6..=15 => RiskLevel::Medium,
            16..=30 => RiskLevel::High,
            _ => RiskLevel::Critical,
        }
    }

    pub fn requires_approval(&self) -> bool {
        matches!(self, RiskLevel::High | RiskLevel::Critical)
    }
}
```

## Context Management

### Token Budget Enforcement

```rust
impl AIDE {
    /// Estimate current token usage
    fn estimate_tokens(&self) -> usize {
        let mut total = 0;

        // System prompt
        total += self.estimate_text_tokens(&self.context.system_prompt);

        // Session summary
        total += self.estimate_text_tokens(&self.context.session_summary);

        // Conversation history
        for msg in &self.messages {
            total += self.estimate_text_tokens(&msg.content);
        }

        // Open files
        for (_, content) in &self.context.open_files {
            total += self.estimate_text_tokens(content);
        }

        total
    }

    /// Summarize old messages to compress context
    async fn summarize_old_messages(&self) -> Result<String> {
        let cutoff = self.messages.len() / 3;
        let old_messages = &self.messages[0..cutoff];

        // Use Claude to generate summary
        let summary_prompt = format!(
            "Summarize this development session concisely (max 500 words):\n\n{}",
            self.format_messages(old_messages)
        );

        let response = self.claude_client.messages.create(
            MessagesRequest {
                model: "claude-haiku-4.5".to_string(),  // Cheap model for summaries
                max_tokens: 1024,
                system: "You summarize development sessions concisely.".to_string(),
                messages: vec![Message {
                    role: "user".to_string(),
                    content: summary_prompt,
                }],
                tools: vec![],
            }
        ).await?;

        Ok(self.extract_text_content(&response))
    }
}
```

## Tools: Next Section

The tool definitions will be in a separate section (coming next).

## Key Design Decisions

**D1:** Single-loop conversational agent (vs Helix's 3-tier)
**D2:** Rush session is source of truth (not agent memory)
**D3:** Token budget enforced with auto-compression
**D4:** Deterministic risk gating (parser-based)
**D5:** Strange Loop: failures → guardrails
**D6:** State persists to `.aide/session-X/` for resumability
**D7:** Performance baseline tracking for anomaly detection
**D8:** Session per conversation (with explicit attach for resume)

## Open Questions

- **Q1:** How to handle multi-day sessions? Auto-compress daily?
- **Q2:** Should guardrails expire after N days?
- **Q3:** Performance anomaly threshold tuning?
- **Q4:** User approval UX - blocking prompt or notification?

---

**Status:** Architecture complete, ready for tool design phase.

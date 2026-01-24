# Human-AI Collaboration Infrastructure Landscape

## Research Date: 2026-01-20

---

## TL;DR

The emerging infrastructure for human-AI collaboration is converging on three layers: **orchestration primitives** (how to coordinate multiple agents), **persistence mechanisms** (how agents maintain state across sessions), and **human-in-the-loop interfaces** (how humans retain meaningful control). The missing infrastructure primitives are largely around durable state, cross-agent communication, and trust calibration systems.

---

## Projects Deep Dive

### Gas Town (Steve Yegge)

**What it is:** A multi-agent orchestration framework built around Claude Code, using a city metaphor for organizing agent work.

**Problems it solves:**
1. **Context loss on restart** - Traditional agents lose all state when they restart. Gas Town persists everything to git.
2. **Coordination chaos** - Managing 4-10 agents manually becomes untenable. Gas Town scales to 20-30.
3. **Work tracking** - Uses "Beads" (git-backed issues with structured IDs like `gt-abc12`) to track work across agents.

**Architecture:**
- **The Mayor**: Coordinator agent that serves as primary interface, decomposes goals into tasks
- **Rigs**: Project containers wrapping git repos with associated agents
- **Hooks**: Git worktrees providing persistent storage that survives restarts
- **Polecats**: Ephemeral worker agents spawned for specific tasks
- **Convoys**: Work tracking units bundling multiple tasks

**Key insight:** The "Propulsion Principle" - using git as the coordination mechanism. Work persists, has version history, enables rollback, and allows multi-agent sharing through standard git operations.

**Recommended workflow (MEOW):** Tell Mayor your goal -> Mayor breaks into tasks -> creates convoy with beads -> spawns agents -> distributes work via hooks -> tracks progress -> summarizes results.

**Deployment:** Gastown-remote enables always-on operation via VPS + Tailscale, avoiding the "laptop closed = agents stopped" problem. Deliberately simple: No Docker, no Kubernetes, just tmux + systemd.

---

### Ralph-TUI

**What it is:** A terminal user interface for orchestrating AI coding agents through autonomous task execution loops.

**Approach to agent coordination:**
1. **Task Selection** - Chooses highest-priority task from queue
2. **Prompt Building** - Constructs contextual instructions
3. **Agent Execution** - Runs AI assistant (Claude Code, OpenCode, Factory Droid)
4. **Completion Detection** - Identifies when tasks finish
5. **Loop Continuation** - Proceeds to next task

**Key features:**
- Supports both simple JSON task lists and Beads (git-backed format with dependencies)
- Session persistence for pause/resume workflows
- Live agent output display with keyboard shortcuts
- Subagent tracing for visibility into nested AI calls
- Remote orchestration across multiple machines
- Skills integration for generating PRDs before autonomous execution

**Design philosophy:** Visibility and control. The TUI provides real-time monitoring while enabling humans to pause, resume, and intervene at any point.

---

### HumanLayer

**What it is:** Two related things under one brand:
1. **HumanLayer SDK** - Python library enabling agents to safely contact humans for approvals, feedback, and help
2. **CodeLayer** - IDE for orchestrating AI coding agents at scale

**HumanLayer SDK solves:**
- **Approval gates**: `@hl.require_approval()` decorator blocks function calls until human review
- **Human-as-tool**: Generic tool allowing agents to ask humans questions mid-execution
- **Channel routing**: Approvals via Slack, Email, Discord

**CodeLayer solves:**
- **Context engineering**: Preventing "chaotic slop-fest" when scaling AI dev across teams
- **Parallel sessions**: MULTICLAU DE runs multiple Claude Code instances with worktrees
- **Keyboard-first workflows**: Speed and control for developers

**Key insight:** The approval mechanism addresses a critical gap - LLMs aren't reliable enough for high-stakes operations without human oversight. But the oversight needs to be lightweight and routed to the right person via channels they already use.

---

## Emerging Patterns in Multi-Agent Systems

### 1. Orchestration Patterns

**Hierarchical coordination:**
- One "coordinator" or "orchestrator" agent that delegates to specialist agents
- Examples: Gas Town Mayor, AutoGen AgentTool, CrewAI orchestrator-workers
- Pattern: Goal decomposition -> task distribution -> result aggregation

**Handoff mechanisms:**
- Agents can transfer control to other agents mid-task
- OpenAI Swarm/Agents SDK: Function returns that include another agent
- CrewAI: Role-based collaboration with dynamic delegation

**Hybrid autonomous + controlled:**
- CrewAI's Crews (autonomous) + Flows (event-driven control)
- Cursor's "autonomy slider" concept
- Pattern: Let agents run freely within bounded zones, humans control zone boundaries

### 2. State Management Patterns

**Git as coordination primitive:**
- Gas Town's Hooks (git worktrees)
- Beads (git-backed issue tracking)
- Key insight: Git already solves distributed state, versioning, and rollback

**Memory architectures:**
- Letta: Memory blocks (human context, persona) that persist across sessions
- LangGraph: Short-term working memory + long-term session persistence
- Pattern: Separate ephemeral context from durable knowledge

**Durable execution:**
- Temporal: Automatic failure handling, state persistence, human-in-the-loop pauses
- Inngest: Step-based workflows with automatic retry
- Pattern: Treat agent work like distributed systems - expect failures, design for recovery

### 3. Human-in-the-Loop Patterns

**Approval gates:**
- HumanLayer's `require_approval` decorator
- LangGraph's state inspection and modification at any point
- Pattern: Deterministic checkpoints where humans can intervene

**Escalation channels:**
- Route approvals to right person via Slack/Email/Discord
- Granular routing by team or individual
- Pattern: Don't block on approvals - async notification + await

**Trust calibration:**
- Cursor's autonomy slider (tab completion -> full agent)
- Devin's human-in-the-loop with engineer approval
- Pattern: Let users control independence level based on task risk

---

## Missing Infrastructure Primitives

### 1. Cross-Agent Communication Standards

**The gap:** Each framework invents its own agent-to-agent messaging. MCP standardizes tool access but not agent coordination.

**What's needed:**
- Standard protocol for agent discovery (who else is working on this?)
- Shared work state (what's been done, what's pending?)
- Conflict resolution (two agents editing same file)
- Capability advertisement (what can this agent do?)

### 2. Trust and Permission Systems

**The gap:** Current systems are binary - either agent has access or doesn't. No nuanced trust model.

**What's needed:**
- Capability-based security for agents (fine-grained permissions)
- Trust scores that evolve with agent track record
- Delegation chains (agent A granted permission by human, can it delegate to agent B?)
- Audit trails that humans can actually review

### 3. Context Management at Scale

**The gap:** Managing what information agents have access to becomes chaotic with teams.

**What's needed:**
- Context boundaries (what can this agent see?)
- Information flow controls (can agent A share this with agent B?)
- Relevance filtering (what subset of context matters for this task?)
- Cross-session context (what did we learn from previous runs?)

### 4. Observability for Agent Systems

**Emerging solutions:** AgentOps, Pydantic Logfire, LangSmith

**What's still needed:**
- Cost attribution across agent chains
- Decision tree visualization (why did agent choose this path?)
- Anomaly detection (agent behaving unexpectedly)
- Performance regression detection

### 5. Durable Identity and State

**The gap:** Agents are ephemeral - they lose identity and learned behaviors on restart.

**What's needed:**
- Persistent agent identity across sessions
- Learned preferences and patterns
- Reputation systems (this agent has good track record on X)
- Handoff protocols (transferring context between agent instances)

---

## OS-Level Primitives for Human-AI Collaboration

### Current State: User-Space Solutions

Everything is built in user-space: processes, files, sockets. No OS awareness that an agent is different from a user.

### Potential OS-Level Primitives

**1. Agent Process Type**
- OS-level distinction between human-driven and agent-driven processes
- Resource quotas and scheduling priorities for agents
- Automatic sandboxing for agent processes
- System call filtering based on agent permissions

**2. Capability-Based File Access**
- Agents get capabilities, not full filesystem access
- Revocable, time-limited, scope-limited access tokens
- Audit logging at kernel level
- Cross-agent sharing through capability transfer

**3. Human Attention Queue**
- OS-level primitive for "need human decision"
- Priority scheduling based on urgency and human availability
- Batching of similar decisions
- Timeout and escalation handling

**4. Shared Memory for Agent Coordination**
- Fast IPC specifically for agent state sharing
- Structured data (not just bytes)
- Change notification and subscription
- Conflict detection and resolution primitives

**5. Time Budget Enforcement**
- Hard limits on agent execution time
- Graceful preemption with state save
- Cost accounting per operation
- Automatic pause when budget exhausted

### Vision: AI-Native Operating System

The trajectory suggests eventually needing OS-level support for:
- Agent lifecycle management (spawn, pause, resume, terminate)
- Resource governance (compute, tokens, time)
- Trust enforcement (what can this agent access?)
- Human attention routing (when to interrupt human?)
- Persistent agent identity (survive reboots, migrations)

---

## Key Players and Their Approaches

| Project | Focus | Key Innovation |
|---------|-------|----------------|
| Gas Town | Multi-agent orchestration | Git as coordination primitive |
| Ralph-TUI | Agent loop management | TUI for visibility and control |
| HumanLayer | Human approval gates | Async approvals via Slack/Email |
| MCP | Tool standardization | "USB-C for AI" - universal tool protocol |
| CrewAI | Team-based agents | Crews (autonomous) + Flows (controlled) |
| LangGraph | Stateful agents | Durable execution with checkpoints |
| AutoGen | Multi-agent conversation | Flexible agent patterns + human input |
| Letta | Persistent memory | Memory blocks that survive sessions |
| OpenAI Agents SDK | Production agents | Handoffs + guardrails + tracing |
| E2B | Sandboxed execution | Secure cloud environments for AI code |
| Temporal | Durable workflows | Reliable execution with human-in-the-loop |
| AgentOps | Observability | Session replay and cost tracking |
| Invariant | Safety guardrails | Rule-based agent behavior constraints |

---

## The Vision These Projects Are Building Toward

### Near-term (2025-2026)
- Reliable multi-agent coordination for development tasks
- Human oversight that doesn't bottleneck agent productivity
- Persistent agent state across sessions
- Cost-effective scaling (Gas Town's 20-30 agents goal)

### Medium-term (2026-2028)
- Agent teams that work like human teams (roles, handoffs, escalation)
- Trust systems that evolve with agent track record
- Context engineering that prevents information overload
- Cross-tool interoperability via MCP and similar protocols

### Long-term Vision
- AI-native computing where agents are first-class OS citizens
- Human-AI collaboration as default, not special case
- Agent ecosystems with marketplaces, reputation, and governance
- Seamless context and state portability across systems

---

## Key Takeaways

1. **Git is emerging as THE coordination primitive** - Gas Town's insight that git solves distributed state, versioning, and rollback is being validated across projects.

2. **Human-in-the-loop needs async channels** - Blocking on human approval kills agent productivity. Route approvals through Slack/Email with timeouts.

3. **Visibility enables trust** - Ralph-TUI and similar tools succeed because they let humans see what agents are doing in real-time.

4. **The orchestrator pattern dominates** - Nearly every framework has some form of coordinator/mayor/orchestrator that manages worker agents.

5. **Durable execution matters** - Agents need to survive failures, pauses, and restarts. Temporal and Inngest patterns are being adopted.

6. **OS-level primitives are the next frontier** - Current solutions are all user-space. Real scale needs kernel-level support for agent processes.

---

## Gaps Worth Exploring

- **Cross-agent communication protocol** - MCP for agents, not just tools
- **Trust and capability systems** - Fine-grained, evolving permissions
- **Human attention scheduling** - When and how to interrupt humans
- **Context boundaries** - What information flows where
- **Agent identity persistence** - Agents that learn and remember

---

## Sources

- Gas Town: github.com/steveyegge/gastown
- Gastown-remote: github.com/numman-ali/gastown-remote
- Ralph-TUI: github.com/subsy/ralph-tui
- HumanLayer: humanlayer.dev, pypi.org/project/humanlayer
- MCP: modelcontextprotocol.io
- CrewAI: github.com/crewAIInc/crewAI
- AutoGen: github.com/microsoft/autogen
- LangGraph: github.com/langchain-ai/langgraph
- Letta: github.com/letta-ai/letta
- OpenAI Agents SDK: github.com/openai/openai-agents-python
- E2B: github.com/e2b-dev/e2b
- Temporal: github.com/temporalio/temporal
- Inngest: github.com/inngest/inngest
- AgentOps: github.com/AgentOps-AI/agentops
- Invariant: github.com/invariantlabs-ai/invariant
- Anthropic: anthropic.com/research/building-effective-agents

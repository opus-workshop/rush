# AI-Native Operating System: Vision Document

**Status:** Research Exploration
**Date:** 2026-01-20

## The Big Picture

We're not building "AI tools." We're exploring what **operating system primitives look like when AI collaboration is assumed from the ground up.**

Current computing infrastructure was designed for:
- Humans as the only source of intent
- Processes as isolated execution units
- Files as the primary state mechanism
- Trust boundaries at user/kernel

AI-native computing needs:
- Intent from humans AND agents
- Agents as first-class process types
- Coordination as a primitive
- Fine-grained, evolving trust systems
- Time-travel for exploration
- Human attention as a schedulable resource

## What We Learned From The Ecosystem

### Gastown (Steve Yegge)
**Innovation:** Git as coordination primitive for multi-agent systems

**Key Insight:** Git already solves:
- Distributed state management
- Versioning and history
- Atomic multi-file operations
- Rollback/branching
- Merge conflict resolution

**Implication:** Don't reinvent state management. Use git worktrees as "agent workspaces."

**Rush Application:**
```bash
rush agent workspace create optimizer
# → git worktree at .rush/workspaces/optimizer/
# Agent works in isolation, Rush merges results
```

---

### HumanLayer
**Innovation:** Async human-in-the-loop that doesn't block agent progress

**Key Insight:** Synchronous approval kills productivity. Route to existing channels (Slack/Email) with:
- Priority levels
- Timeout policies
- Batch approvals for similar requests

**Implication:** Need OS-level "human attention queue" primitive

**Rush Application:**
```bash
rush approval-policy set \
  --destructive "slack://approvals channel" \
  --batch-window "5min" \
  --timeout "1h → auto-reject"
```

---

### Helix (Your Project)
**Innovation:** LLMs at edges, machines in middle

**Key Insights:**
1. **LLMs translate and generate, machines decide and verify**
2. **Specs are contracts** - all work flows from canonical specs
3. **Strange Loop** - failures become guardrails, learned exactly once
4. **Deterministic risk gating** - tag-based scoring, not LLM judgment
5. **Multi-layer security** - whitelist → sandbox → dangerous (VM-only)

**Implication:** Don't use AI for decisions. Use AI for fuzzy→structured translation and structured→code generation.

**Rush Application:**
```bash
# Rush maintains deterministic decision layer
rush execute "risky command"
# → Risk score computed from AST analysis
# → Gate decision is machine logic, not AI inference
# → If approved, execution happens
# → If failed, guardrail learned and persisted
```

---

### ralph-tui
**Innovation:** Visibility and control over agent loops

**Key Insight:** Humans need to **see** what agents are doing and **intervene** when needed.

**Implication:** Agent execution isn't fire-and-forget. It's observable, pauseable, resumable.

**Rush Application:**
```bash
rush agent spawn optimizer --visible
# → TUI shows live output
# → Human can pause/resume/kill
# → Inspect intermediate state
```

---

## The Missing Primitives

Based on this research, here's what **doesn't exist yet** in operating systems:

### 1. Agent Process Type

**Today:** All processes are "user processes" - assumed to be human-initiated

**AI-Native OS Needs:**
```c
// New process type
pid_t spawn_agent(
    char *name,
    capability_set *permissions,
    resource_budget *limits,
    trust_level trust
);

// Agents have:
// - Resource budgets (CPU, memory, API calls, cost)
// - Capability tokens (time-limited, revocable file access)
// - Trust levels (determines what they can do unsupervised)
// - Lifecycle tied to sessions (not just processes)
```

**Rush Implementation:**
```bash
rush agent spawn --budget cpu:1h,cost:$5 --trust low optimizer
# → OS knows this is an agent
# → Sandbox enforced automatically
# → Resource limits hard-enforced
# → Audit trail maintained
```

---

### 2. Coordination Protocol

**Today:** Processes communicate via pipes, sockets, shared memory - low-level

**AI-Native OS Needs:**
```rust
// High-level coordination primitive
pub trait AgentCoordination {
    // Declare intent to other agents
    fn broadcast_intent(&self, intent: Intent);

    // Claim exclusive work
    fn claim_task(&self, task: Task) -> Result<Lock>;

    // Share results
    fn publish_result(&self, result: Result);

    // Request help from specialized agent
    fn delegate(&self, task: Task, to: AgentId);
}
```

**Example:**
```bash
# Agent A working on optimization
rush agent broadcast "optimizing parser, don't touch parser/"

# Agent B sees broadcast
rush agent query-intents
# → "optimizer: working on parser/"

# Agent B delegates
rush agent delegate "review parser changes" reviewer
```

---

### 3. Human Attention Scheduling

**Today:** Humans poll for notifications, or get interrupted randomly

**AI-Native OS Needs:**
```rust
pub struct HumanAttentionQueue {
    // OS-level queue for human decisions
    fn request_approval(
        &self,
        agent: AgentId,
        action: Action,
        priority: Priority,
        timeout: Duration
    ) -> Future<Approval>;

    // Batch similar requests
    fn batch_by_similarity(&self, threshold: f32);

    // Route to appropriate channel
    fn route_to(&self, channel: Channel); // Slack, Email, CLI
}
```

**Rush Implementation:**
```bash
# Configure human attention routing
rush human-queue configure \
  --priority high "cli:blocking" \
  --priority medium "slack:#approvals" \
  --priority low "email:async" \
  --batch-window 5min

# Agent requests approval (non-blocking)
# OS batches similar requests, routes appropriately
# Human approves from Slack, result flows back to agent
```

---

### 4. Capability-Based File Access

**Today:** File permissions are user/group/other - coarse-grained

**AI-Native OS Needs:**
```rust
pub struct CapabilityToken {
    // Fine-grained, revocable, time-limited
    paths: Vec<PathBuf>,        // Specific paths, not whole dirs
    operations: Operations,      // Read, Write, Execute
    expires: Timestamp,          // Time-limited
    conditions: Conditions,      // "Only if tests pass"
}

// Grant token
let token = grant_capability(
    agent_id,
    paths!["/src/parser/mod.rs"],
    Operations::READ | Operations::WRITE,
    Duration::hours(1),
    conditions!["tests_pass"]
);
```

**Rush Implementation:**
```bash
# Grant agent access to specific file
rush capability grant optimizer \
  --path src/parser/mod.rs \
  --ops read,write \
  --duration 1h \
  --condition "cargo test passes"

# Agent tries to write
# → Capability checked
# → Tests run automatically
# → If pass, write allowed
# → If fail, write blocked and human notified
```

---

### 5. Time-Travel Primitives

**Today:** No OS support for checkpointing/rollback (apps do it themselves)

**AI-Native OS Needs:**
```rust
pub trait TimeTravl {
    // Checkpoint entire system state
    fn checkpoint(&self, name: &str) -> SnapshotId;

    // Rollback to previous state
    fn rollback(&self, snapshot: SnapshotId);

    // Branch timeline for exploration
    fn branch(&self, name: &str) -> TimelineId;

    // Compare timelines
    fn diff(&self, a: TimelineId, b: TimelineId) -> Diff;
}
```

**Rush Implementation:**
```bash
# Checkpoint before risky change
rush checkpoint "before refactor"

# Work happens...
cargo test
# → Failures

# Rollback entire state (files + shell env + perf baseline)
rush rollback "before refactor"

# Or branch to try different approach
rush timeline branch "try-different-approach"
# → Separate timeline, original preserved
```

---

### 6. Session as First-Class Primitive

**Today:** Sessions are ad-hoc (tmux, screen) - not OS-native

**AI-Native OS Needs:**
```rust
pub struct Session {
    id: SessionId,
    state: HashMap<String, Value>,  // Persistent key-value
    agents: Vec<AgentId>,            // Attached agents
    checkpoints: Vec<SnapshotId>,    // Timeline
    context: DevelopmentContext,     // Project-aware state
}

// OS maintains sessions, survives reboots
```

**Rush Implementation:**
```bash
# Session persists across reboots
rush session create "parser-optimization"
# → OS maintains session state
# → Agents can attach/detach
# → Context preserved indefinitely

# Next day (after reboot)
rush session attach "parser-optimization"
# → All state restored
# → Agents resume where they left off
```

---

## The Stack: What An AI-Native OS Looks Like

```
┌─────────────────────────────────────────────────────┐
│                 APPLICATIONS                         │
│  (Helix, IDEs, specialized agents)                  │
└─────────────────────────────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────┐
│                  RUSH SHELL                          │
│  • Human-AI orchestration layer                     │
│  • Intent parsing & command synthesis               │
│  • Session management                               │
│  • Agent spawning/coordination                      │
│  • Performance monitoring                           │
│  • Time-travel interface                            │
└─────────────────────────────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────┐
│             AGENT RUNTIME LAYER                      │
│  • Agent process scheduler                          │
│  • Resource budget enforcement                      │
│  • Coordination protocol implementation             │
│  • Human attention queue management                 │
│  • Capability token verification                    │
└─────────────────────────────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────┐
│              KERNEL EXTENSIONS                       │
│  • Agent process type (new syscalls)                │
│  • Capability-based FS (extended permissions)       │
│  • Checkpoint/rollback (COW snapshots)              │
│  • Session persistence (cross-reboot state)         │
│  • Sandboxing (Landlock/seccomp/sandbox-exec)      │
└─────────────────────────────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────┐
│                 BASE OS KERNEL                       │
│  (Linux, macOS, or... custom?)                      │
└─────────────────────────────────────────────────────┘
```

---

## What Rush Becomes In This Vision

**Rush isn't just a shell. It's the orchestration layer for human-AI collaboration.**

### Phase 1: Enhanced Shell (Current)
- Fast startup (<4ms)
- Job control
- Performance monitoring
- Daemon architecture
- POSIX-compatible

### Phase 2: AI-Native Shell (Near-term)
- Agent spawning/management
- Intent-driven commands
- Session persistence
- Time-travel (checkpoint/rollback)
- Coordination protocols

### Phase 3: Orchestration Platform (Medium-term)
- Multi-agent coordination
- Human attention routing
- Capability management
- Cross-session knowledge
- Project-aware context

### Phase 4: OS Integration (Long-term)
- Kernel extensions for agent processes
- Native capability system
- OS-level checkpointing
- Deep system integration

---

## The Path to an AI-Native OS

### Option 1: Linux Extensions
Build Rush on top of Linux with kernel modules for:
- Agent process type (cgroups + BPF)
- Capability tokens (extended attributes)
- Checkpointing (CRIU integration)
- Session persistence (systemd integration)

**Pros:** Leverage existing ecosystem
**Cons:** Constrained by Linux's architecture

### Option 2: macOS Integration
Build Rush on macOS with:
- XPC for agent coordination
- sandbox-exec for capabilities
- Time Machine API for checkpoints
- launchd for session management

**Pros:** Better UX, integrated platform
**Cons:** Closed platform, limited kernel access

### Option 3: Custom Microkernel
Build new OS from scratch:
- Agent-native from ground up
- Clean-slate design
- No legacy constraints

**Pros:** Perfect fit for vision
**Cons:** Massive undertaking, years of work

### Option 4: Hybrid Approach (Recommended)
- **Phase 1-2:** Build Rush on Linux/macOS (userspace only)
- **Phase 3:** Add minimal kernel extensions (eBPF, modules)
- **Phase 4:** Evaluate - sufficient or need custom OS?

---

## Research Questions To Explore

**RQ1: Agent Process Primitives**
What syscalls/APIs are needed for agent processes?
- spawn_agent(), budget_enforce(), capability_check()
- How to integrate with existing schedulers?
- What's the performance overhead?

**RQ2: Coordination Protocols**
How do agents communicate efficiently?
- Shared memory? Message passing? Actor model?
- What's the latency for cross-agent coordination?
- Can we use existing IPC or need new primitives?

**RQ3: Capability Systems**
How fine-grained can we make permissions?
- Per-file vs per-directory?
- Time-limited tokens - how to enforce efficiently?
- Revocation - how to ensure agents can't cache capabilities?

**RQ4: Checkpointing**
What granularity of state needs checkpointing?
- Files only? Environment? Memory? Network state?
- How to make it fast enough for frequent checkpoints?
- CRIU, ZFS snapshots, btrfs COW, or custom?

**RQ5: Human Attention Scheduling**
How to batch and route approval requests?
- ML for similarity clustering?
- Priority inversion problem?
- Timeout policies - what defaults make sense?

**RQ6: Time-Travel Development**
What does it mean to "rewind" a development session?
- Just files, or also shell state, job state, perf baselines?
- Can we efficiently store/restore large states?
- Branch/merge semantics for timelines?

---

## Immediate Next Steps

### 1. Proof of Concept: Rush Agent Runtime (Week 1-2)

Build minimal agent runtime on top of current Rush:

```rust
// rush/src/agent/mod.rs
pub struct AgentRuntime {
    agents: HashMap<AgentId, Agent>,
    coordinator: Coordinator,
    human_queue: HumanAttentionQueue,
}

// Rush spawns agents, manages lifecycle
// Agents execute in sandboxes (Docker initially)
// Coordination via Unix sockets + NDJSON protocol
// Human approvals via CLI (Slack integration later)
```

**Deliverable:**
- `rush agent spawn <name>` works
- Agent can execute commands in sandbox
- Agent can request human approval (blocking CLI prompt)
- Basic resource limits enforced

---

### 2. Prototype: Multi-Agent Coordination (Week 3-4)

Implement coordination protocol:

```rust
// Rush maintains shared coordination state
pub struct CoordinationState {
    intents: Vec<Intent>,           // What agents plan to do
    claims: HashMap<Task, AgentId>, // Who's working on what
    results: Vec<Result>,           // Published outputs
}

// Agents broadcast/query via Rush API
```

**Deliverable:**
- 2 agents coordinate on a task
- No conflicts (exclusive claims work)
- Results shared between agents
- Demonstrate superiority to manual coordination

---

### 3. Research: Checkpoint/Rollback (Week 5-6)

Explore state persistence options:

**Test 1: Git-based**
- Use git worktrees for file state
- Serialize env vars to .rush/checkpoint/<id>/env
- Serialize job state to .rush/checkpoint/<id>/jobs
- Measure: restore time, storage overhead

**Test 2: ZFS snapshots**
- Require ZFS filesystem
- Snapshot entire rush session directory
- Measure: snapshot time, restore time, space usage

**Test 3: BTRFS COW**
- Similar to ZFS but on Linux
- Compare performance

**Deliverable:**
- Benchmark report comparing approaches
- Working prototype of fastest approach
- Demonstrated rollback of complex state

---

### 4. Integration: Helix + Rush (Week 7-8)

Connect Helix's agent orchestration to Rush:

```rust
// Helix agents run in Rush sandboxes
// Rush provides:
// - Process isolation
// - Resource limits
// - Capability tokens
// - Coordination protocol

// Helix provides:
// - Spec-driven workflow
// - Risk gating
// - Strange Loop learning
```

**Deliverable:**
- Helix can spawn agents via Rush
- Rush enforces Helix's security policies
- Demonstrates integration benefits
- Compare to Helix running agents directly

---

## The End Game

**5 years from now, what does this look like?**

Developers work in an AI-native OS where:
- **Intent is the interface** - You say what you want, system figures out how
- **Agents are colleagues** - They work in parallel, coordinate automatically
- **Mistakes are learned** - Guardrails accumulate, system gets safer over time
- **Time is flexible** - Checkpoint/rollback is as natural as undo
- **Context is infinite** - Sessions persist, knowledge accumulates
- **Trust is granular** - Agents earn capabilities through demonstrated reliability
- **Humans are in control** - Final decisions on consequential actions always human

**This is not "AI replaces developers."**

**This is "infrastructure that makes human + AI collaboration 10x more powerful than either alone."**

---

## Open Questions

1. **Is this worth building?** Or is it over-engineering?
2. **Can we prototype enough to prove the value before committing to full OS?**
3. **What's the adoption path?** Who are early adopters?
4. **How does this relate to Rush's immediate goals?** Is this a distraction or the real vision?
5. **What's the business model?** Open source? Commercial OS? Developer tools company?

---

**Status:** This is exploration, not a roadmap. We're figuring out what's possible.

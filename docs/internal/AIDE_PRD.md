# AIDE: AI Development Environment
## Product Requirements Document v0.1

**Status:** Draft
**Date:** 2026-01-20
**Owner:** TBD
**Stakeholders:** TBD

---

## Executive Summary

AIDE (AI Development Environment) is a specialized coding agent optimized for stateful, performance-aware development workflows. Unlike general-purpose AI coding assistants, AIDE maintains persistent development sessions with full context, learns from failures, and provides real-time performance feedback.

**Key Question:** Is this product worth building, or should we just use Claude Code?

**This PRD exists to answer that question rigorously.**

---

## Problem Statement

### Current State of AI Coding Assistants

**Copilots (GitHub Copilot, Cursor Tab):**
- Line-level autocomplete
- Fast, low-friction
- No reasoning, no context beyond current file
- Best for: Boilerplate, obvious patterns

**Chat Interfaces (Claude Code, Aider, Cursor Chat):**
- Natural language interaction
- Whole-file edits
- Stateless: every interaction starts fresh
- Best for: Isolated tasks, bug fixes, feature additions

**Autonomous Agents (Devin, Helix, OpenHands):**
- Spec → Implementation
- Minimal human intervention
- Complex orchestration, expensive
- Best for: Well-defined features with clear specs

### The Gap: Stateful Interactive Development

**Scenario: Performance Debugging Session**

With Claude Code:
```
User: "Run benchmarks"
Claude: [spawns process, waits, shows output]

User: "That's slower than yesterday, investigate"
Claude: [no baseline data, no history]
Claude: "Let me re-run to compare..."
[spawns new process, no session state]

User: "Try optimizing the parser"
Claude: [makes changes]

User: "Test that"
Claude: [spawns new process, environment reset]

User: "Compare to baseline"
Claude: "I don't have baseline data, please run..."
```

Problems:
- No performance history
- No persistent session state
- Can't compare over time
- Restarts from scratch each command
- Manual coordination of tasks

**Scenario: Multi-day Feature Development**

Day 1:
```
User: "Start implementing feature X"
Claude: [works, makes progress]
[User closes Claude Code]
```

Day 2:
```
User: "Continue feature X"
Claude: [reads conversation history - 50k tokens]
Claude: "Let me review what we did..."
[Context limit approaching, old work gets summarized away]
```

Day 5:
```
User: "Continue feature X"
Claude: [conversation history exceeds context limit]
Claude: "I can see we worked on this but details are lost"
[Has to re-read code, re-understand decisions]
```

Problems:
- Context limit forces forgetting
- No persistent memory beyond conversation
- Decisions get lost
- Has to re-learn the codebase

### The Core Problems

**P1: Statelessness**
- Every command starts fresh
- No session memory
- Environment doesn't persist
- Can't build up working state

**P2: Performance Blindness**
- No visibility into execution timing
- Can't detect regressions
- No baseline tracking
- Manual benchmarking required

**P3: Context Limits**
- Long sessions hit token limits
- Early work gets summarized away
- Decisions forgotten
- Knowledge loss over time

**P4: No Learning**
- Makes same mistakes repeatedly
- No memory of what failed
- Can't build guardrails
- Wastes time on known failures

**P5: Sequential Execution**
- One command at a time
- No parallel task management
- Waiting for builds while idle
- Inefficient workflows

### Who Has These Problems?

**Primary Persona: Performance-Conscious System Developer**
- Works on compilers, shells, databases, runtime systems
- Performance matters (latency, throughput)
- Long debugging sessions (multi-hour, multi-day)
- Needs to track regressions
- Frequently benchmarks code changes

**Secondary Persona: Full-Stack Developer (Long Sessions)**
- Building complex features over days
- Needs context to persist
- Makes related changes across many files
- Wants AI to remember decisions
- Frustrated by context limits

**Anti-Persona: Casual Scripter**
- One-off tasks
- Doesn't need session persistence
- Quick fixes, not long workflows
- Claude Code is fine for them

### Market Validation Questions (UNANSWERED)

🔴 **Critical unknowns:**
1. How many developers have multi-day AI coding sessions?
2. How often do they hit context limits?
3. What % of dev time is performance-sensitive work?
4. Would they pay for better tooling?
5. Is Rush adoption a prerequisite or can AIDE work standalone?

**We need data before building.**

---

## Solution Hypothesis

AIDE solves these problems through:

1. **Persistent Sessions**
   - Rush daemon maintains state across invocations
   - Environment, history, jobs persist
   - Resume exactly where you left off

2. **Performance Awareness**
   - Direct access to Rush's perf stats
   - Automatic regression detection
   - Baseline tracking over time

3. **External Memory**
   - State stored in files, not context
   - Summaries, decisions, guardrails persist
   - Selective loading keeps context manageable

4. **Learning from Failures**
   - Strange Loop: errors → guardrails
   - Never repeat same mistake
   - Builds institutional knowledge

5. **Parallel Execution**
   - Background jobs with status tracking
   - Coordinate multiple tasks
   - Efficient resource usage

### Key Assumptions (NEED VALIDATION)

❓ **Assumption 1:** Developers want stateful AI sessions
- **Test:** Survey, user interviews

❓ **Assumption 2:** Performance feedback is valuable during development
- **Test:** Instrument existing workflows, measure how often devs benchmark

❓ **Assumption 3:** Context limits are a real problem
- **Test:** Analyze Claude Code usage logs (if accessible)

❓ **Assumption 4:** Parallel execution saves meaningful time
- **Test:** Benchmark typical workflows (serial vs parallel)

❓ **Assumption 5:** Learned guardrails reduce wasted time
- **Test:** Track repeated failures in existing AI coding sessions

---

## Success Metrics

### North Star Metric
**Time to complete a multi-day feature with AI assistance**
- Includes: Planning, implementation, testing, debugging, optimization
- Measures: End-to-end efficiency including context management

### Supporting Metrics

**Efficiency:**
- Commands per minute (vs Claude Code)
- Time wasted on repeated failures (should → 0 with guardrails)
- Parallel vs serial execution time savings

**Context Management:**
- % of sessions that hit context limits (should be lower)
- Time spent "catching up" at session start (should be lower)
- Knowledge retention across days (measured via quiz/test)

**Performance:**
- Time to detect regression (faster with auto-monitoring)
- Accuracy of regression detection (vs manual)

**Learning:**
- Guardrail effectiveness (% of failures prevented on retry)
- Time until guardrail learned (should be instant)

### Benchmarks (TBD)

**Scenario 1: Performance Regression Hunt**
- Task: Make code change, detect 20% slowdown, diagnose, fix
- Compare: AIDE vs Claude Code vs Manual
- Measure: Time, accuracy, steps required

**Scenario 2: Multi-Day Feature**
- Task: Implement feature over 3 days, 6 hours total
- Compare: AIDE vs Claude Code (context management)
- Measure: Context loss, decision retention, re-work

**Scenario 3: Parallel Build + Test + Lint**
- Task: Make changes, verify all checks pass
- Compare: AIDE (parallel) vs Claude Code (serial)
- Measure: Wall clock time, idle time

---

## User Stories

### Must-Have (MVP)

**US1: Resume Long Session**
> As a developer working on a multi-day feature,
> I want to resume my AI session exactly where I left off,
> So that I don't waste time re-explaining context.

**Acceptance:**
- Start session, make progress, exit
- Resume next day, agent knows what we did
- No need to re-read code or re-explain

**US2: Detect Performance Regression**
> As a performance-conscious developer,
> I want my AI to tell me when code changes make things slower,
> So that I catch regressions immediately.

**Acceptance:**
- Make code change that's 2x slower
- AIDE detects it automatically
- Reports which phase slowed down (lex/parse/execute)

**US3: Never Repeat Failures**
> As a developer using AI,
> I want the AI to learn from mistakes,
> So that it doesn't try the same broken approach twice.

**Acceptance:**
- Command fails with error
- AIDE learns guardrail
- Same command attempted later → blocked with explanation

**US4: Parallel Verification**
> As a developer making changes,
> I want to run tests, build, and lint in parallel,
> So that I get feedback faster.

**Acceptance:**
- Ask AIDE to "verify my changes"
- Spawns 3 background jobs
- Reports results as they complete

### Nice-to-Have (Post-MVP)

**US5: Cross-Session Knowledge**
> As a developer working on multiple features,
> I want knowledge from one session to inform another,
> So that learned patterns are reusable.

**US6: Performance Trend Analysis**
> As a developer optimizing code,
> I want to see performance trends over time,
> So that I can track progress toward goals.

**US7: Collaborative Sessions**
> As a developer pair-programming with AI,
> I want teammates to join my session,
> So that we share context and state.

---

## Feature Requirements

### MVP Features

**F1: Session Management**
- Create session (new or resume)
- Persist state to `.aide/session-X/`
- Load session on resume
- List available sessions

**F2: Rush Integration**
- Execute commands in persistent Rush session
- Query session state (env, jobs, working dir)
- Access performance stats
- Validate syntax before execution

**F3: Agent Loop**
- Token budget enforcement
- Context compression when needed
- Tool execution with risk gating
- State persistence after each turn

**F4: Learning System**
- Detect tool failures
- Generate guardrails
- Persist to `guardrails.md`
- Check guardrails before execution

**F5: Performance Monitoring**
- Query Rush perf stats
- Maintain baseline
- Detect anomalies (>50% deviation)
- Report to user

**F6: Job Control**
- Spawn background jobs
- Query job status
- Wait for completion
- Capture output

### Post-MVP Features

**F7: Multi-Session Knowledge Base**
- Shared guardrails across sessions
- Pattern library
- Cross-session search

**F8: Performance Trends**
- Time-series database
- Trend visualization
- Goal tracking

**F9: Advanced Risk Gating**
- ML-based risk scoring
- User customizable policies
- Audit logs

**F10: Collaboration**
- Session sharing
- Multi-user access
- Conflict resolution

---

## Competitive Analysis

### Claude Code

**Strengths:**
- Official Anthropic product
- Well-integrated with Claude
- Regular updates
- Works with any shell
- Large user base

**Weaknesses:**
- Stateless (subprocess per command)
- No session persistence
- No performance monitoring
- No parallel execution
- Context limits

**AIDE Advantage:**
- Persistent sessions
- Performance awareness
- Job orchestration
- Learning from failures

**Risk:**
- Anthropic could add these features to Claude Code
- Network effects favor established tools

### Cursor

**Strengths:**
- IDE integration
- Copilot + Chat in one
- Large user base
- Agent mode (Cmd+K)

**Weaknesses:**
- Tied to VS Code fork
- No shell integration
- No session persistence beyond project
- Premium pricing

**AIDE Advantage:**
- Shell-native (not IDE-dependent)
- Persistent sessions
- Performance monitoring

**Risk:**
- Most developers prefer IDE workflows
- AIDE's shell focus might be niche

### Aider

**Strengths:**
- Open source
- Works with multiple LLMs
- Git integration
- Focused on code editing

**Weaknesses:**
- No session persistence
- No performance monitoring
- Limited to file operations
- No parallel execution

**AIDE Advantage:**
- All of the above
- Plus Rush integration

**Risk:**
- Aider has established community
- Open source vs commercial unclear for AIDE

### Devin / Helix (Autonomous)

**Strengths:**
- Hands-off operation
- Spec to code
- Complex workflows

**Weaknesses:**
- Expensive ($500/mo for Devin)
- Slow (hours for features)
- Less control
- Not interactive

**AIDE Advantage:**
- Interactive, fast feedback
- Developer in the loop
- Cheaper (API costs only)

**Risk:**
- Different use case
- Not direct competitors

### Summary: Competitive Position

**AIDE's Unique Value:**
1. Persistent sessions (no one else has this)
2. Performance awareness (Rush integration)
3. Learning from failures (Strange Loop)
4. Interactive + stateful (vs autonomous)

**Biggest Risks:**
1. Claude Code adds persistence
2. Market too small (Rush users only?)
3. Developers prefer IDE workflows
4. Hard to explain value vs "just use Claude Code"

---

## Go-to-Market Strategy

### Phase 1: Proof of Concept (Current)
- Build MVP
- Test with Rush users (early adopters)
- Validate core hypotheses
- Gather metrics

### Phase 2: Limited Beta
- 10-20 users (performance-conscious devs)
- Run benchmarks (AIDE vs Claude Code)
- Collect feedback
- Iterate on UX

### Phase 3: Public Launch
- Open source? Commercial? Freemium?
- Documentation, tutorials
- Integration with Rush public launch
- Blog posts, demos, whitepaper

### Phase 4: Growth
- Standalone version (not Rush-dependent)?
- IDE extensions?
- Team/enterprise features?

**Open Question:** What's the business model?
- Free + open source (adoption play)?
- Freemium (free for individuals, paid for teams)?
- Premium only ($20/mo)?
- API wrapper (charge for Claude API + margin)?

---

## MVP Definition

### What's In

✅ Session persistence (create, resume, list)
✅ Rush integration (execute, query state)
✅ Performance monitoring (query stats, detect anomalies)
✅ Learning system (guardrails from failures)
✅ Job control (spawn, query, wait)
✅ REPL interface (interactive CLI)
✅ Token budget management

### What's Out

❌ TUI (use REPL for MVP)
❌ Multi-session knowledge sharing
❌ Performance trend visualization
❌ Collaboration features
❌ IDE integration
❌ Advanced risk policies

### Success Criteria for MVP

**Functional:**
- Can run multi-day session without context loss
- Detects performance regressions automatically
- Learns guardrails from failures (prevents repeats)
- Spawns parallel jobs

**Measurable:**
- Faster than Claude Code on benchmark scenarios (>20% improvement)
- Context retention test: Resume session after 24hrs, agent recalls key decisions
- Zero repeated failures after guardrail learned

**Qualitative:**
- 3/5 beta users prefer AIDE over Claude Code for long sessions
- Users report "feels like pairing with someone who remembers"

---

## Open Questions & Risks

### Product Questions

🔴 **Critical:**
1. Will developers actually want this? (Validate with interviews)
2. Is Rush required or can AIDE work with bash/zsh? (Reduces addressable market)
3. What's the "aha!" moment that hooks users?
4. Can we build this faster than Anthropic adds these features to Claude Code?

🟡 **Important:**
5. REPL vs TUI for MVP? (User preference)
6. Business model? (Open source vs commercial)
7. Standalone product or Rush feature? (Positioning)

### Technical Risks

🔴 **Critical:**
1. Rush daemon stability for long-running sessions
2. Token budget management (will compression degrade quality?)
3. Guardrail accuracy (false positives = frustration)

🟡 **Important:**
4. Performance overhead of AIDE vs direct Rush usage
5. How to handle Rush updates (protocol changes)

### Market Risks

🔴 **Critical:**
1. Market size: How many performance-conscious developers exist?
2. Switching costs: Why leave Claude Code?
3. Network effects: Established tools have momentum

---

## Next Steps

### Immediate (Before Building)

1. **User Research**
   - Interview 10 developers about long AI coding sessions
   - Survey: "Have you hit Claude context limits? How often?"
   - Test: Observe developers using Claude Code, identify pain points

2. **Competitive Benchmarking**
   - Define 3 benchmark scenarios
   - Measure Claude Code performance
   - Establish target improvements

3. **MVP Scope Validation**
   - Show this PRD to potential users
   - Ask: "Would you use this? Why/why not?"
   - Refine based on feedback

### After Validation

4. **Build MVP** (if validation positive)
5. **Private Beta** (10 users)
6. **Measure Metrics** (compare to hypotheses)
7. **Decide:** Ship publicly, iterate, or pivot

---

## Appendix: Hypothetical Whitepaper Outline

**"AI Coding Workflows: A Taxonomy and Comparative Analysis"**

1. **Introduction**
   - Current state of AI-assisted development
   - Problem: One-size-fits-all doesn't work

2. **Taxonomy of AI Coding Tools**
   - Copilots (autocomplete)
   - Chat interfaces (conversational)
   - Autonomous agents (spec-to-code)
   - Stateful assistants (proposed: AIDE)

3. **Metrics That Matter**
   - Time to completion
   - Context retention
   - Learning curve
   - Error recovery
   - Developer satisfaction

4. **Benchmark Scenarios**
   - Scenario 1: Quick bug fix (copilot wins)
   - Scenario 2: Feature implementation (chat wins)
   - Scenario 3: Long debugging session (AIDE wins?)
   - Scenario 4: Performance optimization (AIDE wins?)

5. **Empirical Comparison**
   - Methodology
   - Results
   - Statistical significance

6. **When to Use What**
   - Decision tree
   - Workflow recommendations

7. **Future Directions**
   - Hybrid approaches
   - Specialization vs generalization

**Status:** Outline only. Write after MVP validation.

---

**Document Status:** 🔴 DRAFT - Needs validation before proceeding with implementation.

**Key Decision:** Do we build this, or is Claude Code + hooks sufficient?

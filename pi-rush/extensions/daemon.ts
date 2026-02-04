/**
 * Pi-Rush Daemon Extension
 *
 * Creates a Unix socket server that listens for queries from Rush shell.
 * Implements the Rush ↔ Pi IPC Protocol (JSONL) defined in rush/src/daemon/protocol.rs
 *
 * ## Protocol Overview
 *
 * Rush → Pi (RushToPi):
 * - query: LLM query with shell context
 * - tool_result: Response to a tool call
 *
 * Pi → Rush (PiToRush):
 * - chunk: Streaming content fragment
 * - done: Stream complete
 * - error: Error occurred
 * - tool_call: Pi wants to execute a tool
 *
 * ## Socket Location
 * ~/.pi/rush.sock
 *
 * ## Usage
 * Load this extension and it will automatically start the daemon:
 *   pi -e ./pi-rush/extensions/daemon.ts
 *
 * Or add to extensions in package.json/settings.json for auto-load.
 */

import type { ExtensionAPI, ExtensionContext } from "@mariozechner/pi-coding-agent";
import * as net from "node:net";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import * as readline from "node:readline";

// ============================================================================
// Protocol Types (matching Rush's src/daemon/protocol.rs)
// ============================================================================

/** Shell context passed with queries */
interface ShellContext {
  cwd: string;
  last_command: string | null;
  last_exit_code: number | null;
  history: string[];
  env: Record<string, string>;
}

/** Rush → Pi: Query message */
interface QueryMessage {
  type: "query";
  id: string;
  prompt: string;
  stdin: string | null;
  context: ShellContext;
}

/** Rush → Pi: Tool result message */
interface ToolResultMessage {
  type: "tool_result";
  id: string;
  output: string;
  exit_code: number;
}

/** Rush → Pi: Intent message (? prefix) */
interface IntentMessage {
  type: "intent";
  id: string;
  intent: string;
  context: ShellContext;
  project_type: string | null;
}

type RushToPi = QueryMessage | ToolResultMessage | IntentMessage;

/** Pi → Rush: Streaming chunk */
interface ChunkMessage {
  type: "chunk";
  id: string;
  content: string;
}

/** Pi → Rush: Stream complete */
interface DoneMessage {
  type: "done";
  id: string;
}

/** Pi → Rush: Error occurred */
interface ErrorMessage {
  type: "error";
  id: string;
  message: string;
}

/** Pi → Rush: Tool call request */
interface ToolCallMessage {
  type: "tool_call";
  id: string;
  tool: string;
  args: unknown;
}

/** Pi → Rush: Suggested command for intent query */
interface SuggestedCommandMessage {
  type: "suggested_command";
  id: string;
  command: string;
  explanation: string;
  confidence: number;
}

type PiToRush = ChunkMessage | DoneMessage | ErrorMessage | ToolCallMessage | SuggestedCommandMessage;

// ============================================================================
// Socket Path
// ============================================================================

function getSocketPath(): string {
  const piDir = path.join(os.homedir(), ".pi");
  return path.join(piDir, "rush.sock");
}

function ensurePiDir(): void {
  const piDir = path.join(os.homedir(), ".pi");
  if (!fs.existsSync(piDir)) {
    fs.mkdirSync(piDir, { recursive: true, mode: 0o700 });
  }
}

// ============================================================================
// Session State
// ============================================================================

interface SessionState {
  /** Connection socket */
  socket: net.Socket;
  /** Pending tool call responses (id -> resolve function) */
  pendingToolCalls: Map<string, (result: ToolResultMessage) => void>;
  /** Current request ID being processed */
  currentRequestId: string | null;
  /** Abort controller for current request */
  abortController: AbortController | null;
  /** If set, this is an intent request that needs special response handling */
  pendingIntentId?: string;
}

// ============================================================================
// Daemon Extension
// ============================================================================

export default function rushDaemon(pi: ExtensionAPI) {
  let server: net.Server | null = null;
  const sessions = new Map<net.Socket, SessionState>();
  let extensionCtx: ExtensionContext | null = null;

  // --------------------------------------------------------------------------
  // JSONL encoding/decoding
  // --------------------------------------------------------------------------

  function sendMessage(socket: net.Socket, message: PiToRush): void {
    try {
      const json = JSON.stringify(message);
      socket.write(json + "\n");
    } catch (e) {
      console.error("[rush-daemon] Failed to send message:", e);
    }
  }

  function parseMessage(line: string): RushToPi | null {
    try {
      return JSON.parse(line.trim()) as RushToPi;
    } catch {
      return null;
    }
  }

  // --------------------------------------------------------------------------
  // Build system prompt with shell context
  // --------------------------------------------------------------------------

  function buildContextPrompt(ctx: ShellContext): string {
    const parts: string[] = [];

    parts.push(`Current working directory: ${ctx.cwd}`);

    if (ctx.last_command) {
      parts.push(`Last command: ${ctx.last_command}`);
      if (ctx.last_exit_code !== null) {
        parts.push(`Last exit code: ${ctx.last_exit_code}`);
      }
    }

    if (ctx.history.length > 0) {
      const recentHistory = ctx.history.slice(-10);
      parts.push(`Recent commands:\n${recentHistory.map((c) => `  $ ${c}`).join("\n")}`);
    }

    // Include select environment variables
    const relevantEnvVars = ["SHELL", "USER", "HOME", "PWD", "EDITOR"];
    const envParts = relevantEnvVars
      .filter((v) => ctx.env[v])
      .map((v) => `  ${v}=${ctx.env[v]}`);
    if (envParts.length > 0) {
      parts.push(`Environment:\n${envParts.join("\n")}`);
    }

    return parts.join("\n");
  }

  // --------------------------------------------------------------------------
  // Handle incoming messages
  // --------------------------------------------------------------------------

  async function handleQuery(
    session: SessionState,
    msg: QueryMessage
  ): Promise<void> {
    session.currentRequestId = msg.id;
    session.abortController = new AbortController();

    try {
      // Build the prompt with context
      let fullPrompt = msg.prompt;

      // If there's stdin content, prepend it
      if (msg.stdin) {
        fullPrompt = `<stdin>\n${msg.stdin}\n</stdin>\n\n${msg.prompt}`;
      }

      // Add shell context as a preamble
      const contextInfo = buildContextPrompt(msg.context);
      fullPrompt = `<shell_context>\n${contextInfo}\n</shell_context>\n\n${fullPrompt}`;

      // Send the message to Pi
      // Check if agent is idle first, if not queue as follow-up
      if (extensionCtx?.isIdle()) {
        pi.sendUserMessage(fullPrompt);
      } else {
        // Agent is busy, queue as follow-up
        pi.sendUserMessage(fullPrompt, { deliverAs: "followUp" });
      }

    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      sendMessage(session.socket, {
        type: "error",
        id: msg.id,
        message: errorMsg,
      });
      session.currentRequestId = null;
      session.abortController = null;
    }
  }

  function handleToolResult(
    session: SessionState,
    msg: ToolResultMessage
  ): void {
    const resolver = session.pendingToolCalls.get(msg.id);
    if (resolver) {
      resolver(msg);
      session.pendingToolCalls.delete(msg.id);
    }
  }

  /**
   * Handle intent-to-command messages (? prefix)
   *
   * The intent is converted to a shell command by Pi with the help of:
   * - Shell context (cwd, history, etc.)
   * - Project type (rust, node, python, etc.)
   */
  async function handleIntent(
    session: SessionState,
    msg: IntentMessage
  ): Promise<void> {
    session.currentRequestId = msg.id;
    session.abortController = new AbortController();

    try {
      // Build the intent prompt with context
      const contextInfo = buildContextPrompt(msg.context);

      // Build project-aware prompt
      const projectInfo = msg.project_type
        ? `This is a ${msg.project_type} project.`
        : "";

      const intentPrompt = `Convert this natural language intent to a shell command.

<shell_context>
${contextInfo}
</shell_context>

${projectInfo}

User intent: "${msg.intent}"

Respond with ONLY a JSON object in this exact format (no markdown, no explanation):
{"command": "the shell command", "explanation": "brief explanation of what it does", "confidence": 0.95}

Guidelines:
- Generate a single, complete shell command
- Use appropriate flags and options
- Consider the project type and context
- Confidence should be 0.0-1.0 (lower if intent is ambiguous)
- explanation should be 1 sentence
- Do NOT include markdown code blocks
- Do NOT include any text before or after the JSON`;

      // Mark this as an intent request so we handle the response specially
      session.pendingIntentId = msg.id;

      // Send the message to Pi
      if (extensionCtx?.isIdle()) {
        pi.sendUserMessage(intentPrompt);
      } else {
        pi.sendUserMessage(intentPrompt, { deliverAs: "followUp" });
      }

    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      sendMessage(session.socket, {
        type: "error",
        id: msg.id,
        message: errorMsg,
      });
      session.currentRequestId = null;
      session.abortController = null;
      session.pendingIntentId = undefined;
    }
  }

  function handleMessage(session: SessionState, msg: RushToPi): void {
    switch (msg.type) {
      case "query":
        handleQuery(session, msg);
        break;
      case "tool_result":
        handleToolResult(session, msg);
        break;
      case "intent":
        handleIntent(session, msg);
        break;
    }
  }

  // --------------------------------------------------------------------------
  // Connection handling
  // --------------------------------------------------------------------------

  function handleConnection(socket: net.Socket): void {
    console.log("[rush-daemon] Client connected");

    const session: SessionState = {
      socket,
      pendingToolCalls: new Map(),
      currentRequestId: null,
      abortController: null,
    };
    sessions.set(socket, session);

    // Set up line-by-line reading for JSONL
    const rl = readline.createInterface({
      input: socket,
      crlfDelay: Infinity,
    });

    rl.on("line", (line) => {
      const msg = parseMessage(line);
      if (msg) {
        handleMessage(session, msg);
      } else {
        console.error("[rush-daemon] Failed to parse message:", line);
      }
    });

    socket.on("close", () => {
      console.log("[rush-daemon] Client disconnected");
      sessions.delete(socket);
      // Abort any pending request
      if (session.abortController) {
        session.abortController.abort();
      }
    });

    socket.on("error", (err) => {
      console.error("[rush-daemon] Socket error:", err);
      sessions.delete(socket);
    });
  }

  // --------------------------------------------------------------------------
  // Server lifecycle
  // --------------------------------------------------------------------------

  function startServer(): void {
    const socketPath = getSocketPath();

    // Clean up existing socket
    try {
      if (fs.existsSync(socketPath)) {
        fs.unlinkSync(socketPath);
      }
    } catch {
      // Ignore errors
    }

    ensurePiDir();

    server = net.createServer(handleConnection);

    server.on("error", (err) => {
      console.error("[rush-daemon] Server error:", err);
    });

    server.listen(socketPath, () => {
      // Set socket permissions (owner only)
      try {
        fs.chmodSync(socketPath, 0o600);
      } catch {
        // Ignore on Windows
      }
      console.log(`[rush-daemon] Listening on ${socketPath}`);
    });
  }

  function stopServer(): void {
    if (server) {
      // Close all client connections
      for (const [socket, session] of sessions) {
        if (session.abortController) {
          session.abortController.abort();
        }
        socket.destroy();
      }
      sessions.clear();

      server.close();
      server = null;

      // Clean up socket file
      const socketPath = getSocketPath();
      try {
        if (fs.existsSync(socketPath)) {
          fs.unlinkSync(socketPath);
        }
      } catch {
        // Ignore errors
      }

      console.log("[rush-daemon] Server stopped");
    }
  }

  // --------------------------------------------------------------------------
  // Event handlers for streaming responses back to Rush
  // --------------------------------------------------------------------------

  // Hook into agent events to stream responses
  pi.on("agent_start", async (_event, ctx) => {
    extensionCtx = ctx;
  });

  // Stream text chunks to connected Rush clients
  pi.on("turn_end", async (event, ctx) => {
    // Find sessions with active requests
    for (const [_socket, session] of sessions) {
      if (session.currentRequestId) {
        // Extract text content from the message
        const message = event.message;
        let fullText = "";

        if (message?.role === "assistant" && message.content) {
          for (const block of message.content) {
            if (block.type === "text" && block.text) {
              fullText += block.text;
            }
          }
        }

        // Check if this is an intent response
        if (session.pendingIntentId) {
          // Parse the response as JSON to extract command suggestion
          try {
            // Try to extract JSON from the response (might have extra text)
            const jsonMatch = fullText.match(/\{[\s\S]*?\}/);
            if (jsonMatch) {
              const parsed = JSON.parse(jsonMatch[0]) as {
                command?: string;
                explanation?: string;
                confidence?: number;
              };

              if (parsed.command) {
                sendMessage(session.socket, {
                  type: "suggested_command",
                  id: session.pendingIntentId,
                  command: parsed.command,
                  explanation: parsed.explanation || "Generated command",
                  confidence: typeof parsed.confidence === "number" ? parsed.confidence : 0.8,
                });
              } else {
                sendMessage(session.socket, {
                  type: "error",
                  id: session.pendingIntentId,
                  message: "Failed to generate command: no command in response",
                });
              }
            } else {
              // No JSON found, try to use the text as a command
              const trimmedText = fullText.trim();
              if (trimmedText && !trimmedText.includes("\n")) {
                // Single line, might be a command
                sendMessage(session.socket, {
                  type: "suggested_command",
                  id: session.pendingIntentId,
                  command: trimmedText,
                  explanation: "Generated command",
                  confidence: 0.6,
                });
              } else {
                sendMessage(session.socket, {
                  type: "error",
                  id: session.pendingIntentId,
                  message: "Failed to parse command from response",
                });
              }
            }
          } catch (e) {
            sendMessage(session.socket, {
              type: "error",
              id: session.pendingIntentId,
              message: `Failed to parse response: ${e instanceof Error ? e.message : String(e)}`,
            });
          }

          // Send done message
          sendMessage(session.socket, {
            type: "done",
            id: session.pendingIntentId,
          });

          session.pendingIntentId = undefined;
        } else {
          // Regular query - stream the text
          if (fullText) {
            sendMessage(session.socket, {
              type: "chunk",
              id: session.currentRequestId,
              content: fullText,
            });
          }

          // Send done message
          sendMessage(session.socket, {
            type: "done",
            id: session.currentRequestId,
          });
        }

        session.currentRequestId = null;
        session.abortController = null;
      }
    }
  });

  // Handle tool calls - forward to Rush for execution
  pi.on("tool_call", async (event, ctx) => {
    // Only intercept bash calls to route through Rush
    if (event.toolName !== "bash") {
      return; // Let Pi handle other tools normally
    }

    // Find the active session
    for (const [_socket, session] of sessions) {
      if (session.currentRequestId) {
        const toolCallId = `tool-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;

        // Send tool call to Rush
        sendMessage(session.socket, {
          type: "tool_call",
          id: toolCallId,
          tool: event.toolName,
          args: event.input,
        });

        // Wait for response (with timeout)
        const result = await new Promise<ToolResultMessage | null>((resolve) => {
          const timeout = setTimeout(() => {
            session.pendingToolCalls.delete(toolCallId);
            resolve(null);
          }, 60000); // 60 second timeout

          session.pendingToolCalls.set(toolCallId, (result) => {
            clearTimeout(timeout);
            resolve(result);
          });
        });

        if (result) {
          // Return the tool result to Pi
          return {
            content: [{ type: "text", text: result.output }],
            details: { exitCode: result.exit_code },
            isError: result.exit_code !== 0,
          };
        } else {
          return {
            block: true,
            reason: "Tool call timed out waiting for Rush response",
          };
        }
      }
    }
  });

  // --------------------------------------------------------------------------
  // Extension lifecycle
  // --------------------------------------------------------------------------

  // Start server when session begins
  pi.on("session_start", async (_event, ctx) => {
    extensionCtx = ctx;
    startServer();
    if (ctx.hasUI) {
      ctx.ui.notify("Rush daemon started", "info");
    }
  });

  // Stop server on shutdown
  pi.on("session_shutdown", async () => {
    stopServer();
  });

  // Register /pi-daemon command for manual control
  pi.registerCommand("pi-daemon", {
    description: "Control Pi↔Rush IPC daemon (start/stop/status)",
    handler: async (args, ctx) => {
      const action = args?.trim() || "status";

      switch (action) {
        case "start":
          if (server) {
            ctx.ui.notify("Rush daemon already running", "warning");
          } else {
            startServer();
            ctx.ui.notify("Rush daemon started", "success");
          }
          break;

        case "stop":
          if (server) {
            stopServer();
            ctx.ui.notify("Rush daemon stopped", "info");
          } else {
            ctx.ui.notify("Rush daemon not running", "warning");
          }
          break;

        case "status":
        default: {
          const socketPath = getSocketPath();
          const isRunning = server !== null;
          const clientCount = sessions.size;
          const status = isRunning
            ? `Running at ${socketPath} (${clientCount} clients)`
            : "Not running";
          ctx.ui.notify(`Rush daemon: ${status}`, "info");
          break;
        }
      }
    },
  });
}

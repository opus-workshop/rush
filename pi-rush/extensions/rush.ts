/**
 * Pi-Rush Extension
 *
 * Integrates Rush shell with Pi for faster command execution and structured output.
 *
 * Features:
 * - `rush` tool: Execute commands with JSON output
 * - `rush_git` tool: Fast native git operations
 * - `rush_find` tool: Parallel file search with .gitignore awareness
 * - Daemon mode support for 0.4ms startup latency
 * - Automatic JSON parsing of structured output
 *
 * Usage:
 *   pi install git:github.com/paiml/rush --path pi-rush
 *   # Or for development:
 *   pi -e ./pi-rush/extensions/rush.ts
 */

import type { ExtensionAPI, ExtensionContext } from "@mariozechner/pi-coding-agent";
import {
  truncateTail,
  DEFAULT_MAX_BYTES,
  DEFAULT_MAX_LINES,
  formatSize,
} from "@mariozechner/pi-coding-agent";
import { Type, type Static } from "@sinclair/typebox";
import { StringEnum } from "@mariozechner/pi-ai";
import { Text } from "@mariozechner/pi-tui";

// Tool input types
const RushCommandInput = Type.Object({
  command: Type.String({ description: "Shell command to execute via Rush" }),
  json: Type.Optional(Type.Boolean({ description: "Request JSON output (default: true)" })),
  timeout: Type.Optional(Type.Number({ description: "Timeout in seconds" })),
});
type RushCommandInput = Static<typeof RushCommandInput>;

const RushGitInput = Type.Object({
  operation: StringEnum(["status", "log", "diff", "branch"] as const),
  args: Type.Optional(Type.String({ description: "Additional arguments" })),
});
type RushGitInput = Static<typeof RushGitInput>;

const RushFindInput = Type.Object({
  pattern: Type.Optional(Type.String({ description: "Glob pattern to match" })),
  path: Type.Optional(Type.String({ description: "Starting path (default: .)" })),
  type: Type.Optional(StringEnum(["f", "d", "all"] as const)),
  name: Type.Optional(Type.String({ description: "Name pattern to match" })),
  maxDepth: Type.Optional(Type.Number({ description: "Maximum directory depth" })),
});
type RushFindInput = Static<typeof RushFindInput>;

const RushGrepInput = Type.Object({
  pattern: Type.String({ description: "Search pattern (regex)" }),
  path: Type.Optional(Type.String({ description: "Path or glob to search" })),
  ignoreCase: Type.Optional(Type.Boolean({ description: "Case-insensitive search" })),
  context: Type.Optional(Type.Number({ description: "Lines of context around matches" })),
});
type RushGrepInput = Static<typeof RushGrepInput>;

// Check if Rush is available
async function checkRush(pi: ExtensionAPI): Promise<{ available: boolean; version?: string; daemon?: boolean }> {
  try {
    const result = await pi.exec("rush", ["--version"], { timeout: 2000 });
    if (result.code === 0) {
      const version = result.stdout.trim();
      // Check if daemon is running
      const daemonCheck = await pi.exec("rush", ["-c", "echo ok"], { timeout: 1000 });
      return { available: true, version, daemon: daemonCheck.code === 0 };
    }
  } catch {
    // Rush not available
  }
  return { available: false };
}

// Execute a Rush command
async function execRush(
  pi: ExtensionAPI,
  command: string,
  options: { json?: boolean; timeout?: number; signal?: AbortSignal } = {}
): Promise<{ stdout: string; stderr: string; code: number; parsed?: unknown }> {
  const { json = true, timeout, signal } = options;

  const args = ["-c", command];
  const env = json ? { RUSH_ERROR_FORMAT: "json" } : undefined;

  const result = await pi.exec("rush", args, { timeout: timeout ? timeout * 1000 : undefined, signal, env });

  let parsed: unknown;
  if (json && result.stdout) {
    try {
      parsed = JSON.parse(result.stdout);
    } catch {
      // Not valid JSON, that's okay
    }
  }

  return { ...result, parsed };
}

export default function rushExtension(pi: ExtensionAPI) {
  let rushAvailable = false;
  let rushVersion: string | undefined;
  let daemonRunning = false;

  // Check Rush availability on startup
  pi.on("session_start", async (_event, ctx) => {
    const status = await checkRush(pi);
    rushAvailable = status.available;
    rushVersion = status.version;
    daemonRunning = status.daemon ?? false;

    if (rushAvailable && ctx.hasUI) {
      const mode = daemonRunning ? "daemon" : "direct";
      ctx.ui.notify(`Rush ${rushVersion} (${mode})`, "info");
    } else if (!rushAvailable && ctx.hasUI) {
      ctx.ui.notify("Rush not found - install from github.com/paiml/rush", "warning");
    }
  });

  // Register rush command tool
  pi.registerTool({
    name: "rush",
    label: "Rush",
    description: `Execute shell commands via Rush shell with optional JSON output.
Rush provides 17-427x faster built-in commands than bash/zsh.
Use for: ls, cat, grep, find, git operations, and general shell commands.
JSON mode returns structured data for easier parsing.`,
    parameters: RushCommandInput,

    async execute(toolCallId, params, signal, onUpdate, ctx) {
      if (!rushAvailable) {
        return {
          content: [{ type: "text", text: "Error: Rush shell is not installed. Install from https://github.com/paiml/rush" }],
          details: { error: "rush_not_found" },
          isError: true,
        };
      }

      const { command, json = true, timeout } = params;

      onUpdate?.({
        content: [{ type: "text", text: `Running: ${command}` }],
        details: { status: "running" },
      });

      const result = await execRush(pi, command, { json, timeout, signal });

      // Truncate if needed
      const truncation = truncateTail(result.stdout, {
        maxLines: DEFAULT_MAX_LINES,
        maxBytes: DEFAULT_MAX_BYTES,
      });

      let output = truncation.content;
      if (truncation.truncated) {
        output += `\n\n[Output truncated: showing last ${truncation.outputLines} of ${truncation.totalLines} lines`;
        output += ` (${formatSize(truncation.outputBytes)} of ${formatSize(truncation.totalBytes)})]`;
      }

      if (result.code !== 0) {
        output += `\n[Exit code: ${result.code}]`;
        if (result.stderr) {
          output += `\n[stderr: ${result.stderr.trim()}]`;
        }
      }

      return {
        content: [{ type: "text", text: output }],
        details: {
          command,
          exitCode: result.code,
          json: result.parsed,
          truncated: truncation.truncated,
        },
        isError: result.code !== 0,
      };
    },

    renderCall(args, theme) {
      let text = theme.fg("toolTitle", theme.bold("rush "));
      text += theme.fg("code", args.command);
      if (args.json === false) {
        text += theme.fg("muted", " (text mode)");
      }
      return new Text(text, 0, 0);
    },

    renderResult(result, { expanded }, theme) {
      const details = result.details as { exitCode?: number; json?: unknown; truncated?: boolean } | undefined;
      const exitCode = details?.exitCode ?? 0;

      let text = "";
      if (exitCode === 0) {
        text += theme.fg("success", "✓ ");
      } else {
        text += theme.fg("error", `✗ Exit ${exitCode} `);
      }

      // Show JSON indicator if we got structured output
      if (details?.json) {
        text += theme.fg("accent", "[JSON] ");
      }
      if (details?.truncated) {
        text += theme.fg("warning", "[truncated] ");
      }

      // Show output
      const content = result.content?.[0];
      if (content?.type === "text" && content.text) {
        const lines = content.text.split("\n");
        const preview = lines.slice(0, expanded ? 50 : 5);
        text += "\n" + theme.fg("dim", preview.join("\n"));
        if (!expanded && lines.length > 5) {
          text += theme.fg("muted", `\n... (${lines.length - 5} more lines)`);
        }
      }

      return new Text(text, 0, 0);
    },
  });

  // Register rush_git tool for fast git operations
  pi.registerTool({
    name: "rush_git",
    label: "Rush Git",
    description: `Fast native git operations via Rush (5-10x faster than git CLI).
Operations: status, log, diff, branch. Always returns JSON.`,
    parameters: RushGitInput,

    async execute(toolCallId, params, signal, onUpdate, ctx) {
      if (!rushAvailable) {
        return {
          content: [{ type: "text", text: "Error: Rush not installed" }],
          isError: true,
        };
      }

      const { operation, args = "" } = params;
      const command = `git ${operation} --json ${args}`.trim();

      const result = await execRush(pi, command, { json: true, signal });

      return {
        content: [{ type: "text", text: result.stdout || "(no output)" }],
        details: {
          operation,
          exitCode: result.code,
          json: result.parsed,
        },
        isError: result.code !== 0,
      };
    },

    renderCall(args, theme) {
      let text = theme.fg("toolTitle", theme.bold("rush_git "));
      text += theme.fg("accent", args.operation);
      if (args.args) {
        text += " " + theme.fg("dim", args.args);
      }
      return new Text(text, 0, 0);
    },
  });

  // Register rush_find tool for fast file search
  pi.registerTool({
    name: "rush_find",
    label: "Rush Find",
    description: `Parallel file search with .gitignore awareness.
Much faster than GNU find. Automatically respects .gitignore.`,
    parameters: RushFindInput,

    async execute(toolCallId, params, signal, onUpdate, ctx) {
      if (!rushAvailable) {
        return {
          content: [{ type: "text", text: "Error: Rush not installed" }],
          isError: true,
        };
      }

      const { pattern, path = ".", type, name, maxDepth } = params;

      let command = `find ${path}`;
      if (type && type !== "all") {
        command += ` -type ${type}`;
      }
      if (name) {
        command += ` -name "${name}"`;
      }
      if (maxDepth !== undefined) {
        command += ` -maxdepth ${maxDepth}`;
      }
      if (pattern) {
        command += ` -path "${pattern}"`;
      }
      command += " --json";

      const result = await execRush(pi, command, { json: true, signal });

      const truncation = truncateTail(result.stdout, {
        maxLines: DEFAULT_MAX_LINES,
        maxBytes: DEFAULT_MAX_BYTES,
      });

      return {
        content: [{ type: "text", text: truncation.content }],
        details: {
          exitCode: result.code,
          json: result.parsed,
          truncated: truncation.truncated,
        },
        isError: result.code !== 0,
      };
    },
  });

  // Register rush_grep tool for fast text search
  pi.registerTool({
    name: "rush_grep",
    label: "Rush Grep",
    description: `Ripgrep-powered text search (10-50x faster than grep).
Searches recursively with .gitignore awareness.`,
    parameters: RushGrepInput,

    async execute(toolCallId, params, signal, onUpdate, ctx) {
      if (!rushAvailable) {
        return {
          content: [{ type: "text", text: "Error: Rush not installed" }],
          isError: true,
        };
      }

      const { pattern, path = ".", ignoreCase, context } = params;

      let command = `grep "${pattern}" ${path}`;
      if (ignoreCase) {
        command += " -i";
      }
      if (context !== undefined) {
        command += ` -C ${context}`;
      }
      command += " --json";

      const result = await execRush(pi, command, { json: true, signal });

      const truncation = truncateTail(result.stdout, {
        maxLines: DEFAULT_MAX_LINES,
        maxBytes: DEFAULT_MAX_BYTES,
      });

      return {
        content: [{ type: "text", text: truncation.content }],
        details: {
          pattern,
          exitCode: result.code,
          json: result.parsed,
          truncated: truncation.truncated,
        },
        isError: result.code !== 0,
      };
    },
  });

  // Register /rush-daemon command to control daemon
  pi.registerCommand("rush-daemon", {
    description: "Start/stop Rush daemon for faster execution",
    handler: async (args, ctx) => {
      const action = args?.trim() || "status";

      if (action === "start") {
        const result = await pi.exec("rushd", ["start"], { timeout: 5000 });
        if (result.code === 0) {
          daemonRunning = true;
          ctx.ui.notify("Rush daemon started (0.4ms latency)", "success");
        } else {
          ctx.ui.notify(`Failed to start daemon: ${result.stderr}`, "error");
        }
      } else if (action === "stop") {
        const result = await pi.exec("rushd", ["stop"], { timeout: 5000 });
        daemonRunning = false;
        ctx.ui.notify("Rush daemon stopped", "info");
      } else {
        const status = daemonRunning ? "running" : "stopped";
        ctx.ui.notify(`Rush daemon: ${status}`, "info");
      }
    },
  });

  // Register /rush-status command
  pi.registerCommand("rush-status", {
    description: "Show Rush shell status",
    handler: async (_args, ctx) => {
      if (!rushAvailable) {
        ctx.ui.notify("Rush not installed", "warning");
        return;
      }

      const mode = daemonRunning ? "daemon (0.4ms)" : "direct (4.9ms)";
      ctx.ui.notify(`Rush ${rushVersion} - ${mode}`, "info");
    },
  });
}

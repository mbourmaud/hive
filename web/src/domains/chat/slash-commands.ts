import type { RightSidebarTab } from "@/domains/monitor/store";
import type { DroneInfo } from "@/domains/monitor/types";
import type { ChatAction } from "./types";

// ── Context passed to every slash command handler ────────────────────────────

export interface SlashCommandContext {
  toast: (message: string, variant: "info" | "success" | "error") => void;
  dispatchChat: (action: ChatAction) => void;
  selectedModel: string | null;
  setSelectedModel: (model: string) => void;
  setSessionsModalOpen: (open: boolean) => void;
  handleNewSession: () => void;
  resetSession: () => void;
  drones: DroneInfo[];
  rightSidebarCollapsed: boolean;
  openRightSidebar: (tab: RightSidebarTab) => void;
  activeSessionId: string | null;
  reloadSession: (id: string) => void;
}

// ── Command definition ──────────────────────────────────────────────────────

interface SlashCommandDef {
  name: string;
  aliases?: string[];
  execute: (args: string[], ctx: SlashCommandContext) => Promise<void> | void;
}

// ── Command handlers ────────────────────────────────────────────────────────

function handleNew(_args: string[], ctx: SlashCommandContext): void {
  ctx.handleNewSession();
}

function handleClear(_args: string[], ctx: SlashCommandContext): void {
  ctx.resetSession();
  ctx.toast("Conversation cleared", "info");
}

async function handleCompact(_args: string[], ctx: SlashCommandContext): Promise<void> {
  if (!ctx.activeSessionId) {
    ctx.toast("No active session", "error");
    return;
  }
  ctx.toast("Compacting conversation...", "info");
  try {
    const res = await fetch(`/api/chat/sessions/${ctx.activeSessionId}/compact`, {
      method: "POST",
    });
    if (res.ok) {
      ctx.toast("Conversation compacted", "success");
      ctx.reloadSession(ctx.activeSessionId);
    } else {
      const text = await res.text();
      ctx.toast(`Compact failed: ${text}`, "error");
    }
  } catch (err) {
    const msg = err instanceof Error ? err.message : "Unknown error";
    ctx.toast(`Compact failed: ${msg}`, "error");
  }
}

function handleModel(args: string[], ctx: SlashCommandContext): void {
  const modelName = args[0];
  if (modelName) {
    ctx.setSelectedModel(modelName);
    ctx.toast(`Model switched to ${modelName}`, "success");
  } else {
    ctx.toast("Usage: /model <name> (e.g. /model sonnet)", "info");
  }
}

function handleSteps(_args: string[], ctx: SlashCommandContext): void {
  ctx.toast("Toggle steps via the steps button on each turn", "info");
}

function handleUndo(_args: string[], ctx: SlashCommandContext): void {
  ctx.toast("Undo is not yet implemented", "info");
}

function handleSessions(_args: string[], ctx: SlashCommandContext): void {
  ctx.setSessionsModalOpen(true);
}

function handleHelp(_args: string[], ctx: SlashCommandContext): void {
  ctx.toast(
    "Commands: /new, /clear, /sessions, /model <name>, /launch <name> <prompt>, /status, /stop <name>, /logs <name>, /help",
    "info",
  );
}

async function handleLaunch(args: string[], ctx: SlashCommandContext): Promise<void> {
  const droneName = args[0];
  const prompt = args.slice(1).join(" ");
  if (!droneName || !prompt) {
    ctx.toast("Usage: /launch <name> <prompt>", "info");
    return;
  }
  ctx.toast(`Launching drone '${droneName}'...`, "info");
  try {
    const res = await fetch("/api/drones/launch", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        name: droneName,
        prompt,
        model: ctx.selectedModel ?? "sonnet",
        mode: "agent-team",
      }),
    });
    if (res.ok) {
      ctx.toast(`Drone '${droneName}' launched`, "success");
      ctx.dispatchChat({ type: "DRONE_LAUNCHED", droneName, prompt });
      if (ctx.rightSidebarCollapsed) ctx.openRightSidebar("drones");
    } else {
      const text = await res.text();
      ctx.toast(`Failed to launch: ${text}`, "error");
    }
  } catch (err) {
    const msg = err instanceof Error ? err.message : "Unknown error";
    ctx.toast(`Launch failed: ${msg}`, "error");
  }
}

function handleStatus(_args: string[], ctx: SlashCommandContext): void {
  const { drones } = ctx;
  if (drones.length === 0) {
    ctx.toast("No drones detected", "info");
    return;
  }
  const summary = drones
    .map((d) => `${d.name}: ${d.liveness} (${d.progress[0]}/${d.progress[1]})`)
    .join(", ");
  ctx.toast(`Drones: ${summary}`, "info");

  const activeDrones = drones.filter((d) => d.liveness === "working");
  if (ctx.rightSidebarCollapsed && activeDrones.length > 0) ctx.openRightSidebar("drones");
}

async function handleStop(args: string[], ctx: SlashCommandContext): Promise<void> {
  const targetDrone = args[0];
  if (!targetDrone) {
    ctx.toast("Usage: /stop <name>", "info");
    return;
  }
  try {
    const res = await fetch(`/api/drones/${targetDrone}/stop`, { method: "POST" });
    if (res.ok) {
      ctx.toast(`Drone '${targetDrone}' stopped`, "success");
    } else {
      const text = await res.text();
      ctx.toast(`Failed to stop: ${text}`, "error");
    }
  } catch (err) {
    const msg = err instanceof Error ? err.message : "Unknown error";
    ctx.toast(`Stop failed: ${msg}`, "error");
  }
}

function handleLogs(args: string[], ctx: SlashCommandContext): void {
  const droneName = args[0];
  if (!droneName) {
    ctx.toast("Usage: /logs <name>", "info");
    return;
  }
  const drone = ctx.drones.find((d) => d.name === droneName);
  if (!drone) {
    ctx.toast(`Drone '${droneName}' not found`, "error");
    return;
  }
  const taskSummary = drone.tasks
    .map((t) => `  ${t.status === "completed" ? "[x]" : "[ ]"} ${t.subject}`)
    .join("\n");
  const memberSummary = drone.members
    .map((m) => `  ${m.name} (${m.agent_type}) — ${m.liveness}`)
    .join("\n");
  const recent = drone.messages
    .slice(-3)
    .map((m) => `  [${m.from}\u2192${m.to}] ${m.text.slice(0, 80)}`)
    .join("\n");
  const info = [
    `Drone: ${drone.name} (${drone.liveness})`,
    `Progress: ${drone.progress[0]}/${drone.progress[1]} tasks`,
    `Elapsed: ${drone.elapsed}`,
    `Cost: $${drone.cost.total_usd.toFixed(2)}`,
    taskSummary ? `\nTasks:\n${taskSummary}` : "",
    memberSummary ? `\nAgents:\n${memberSummary}` : "",
    recent ? `\nRecent messages:\n${recent}` : "",
  ]
    .filter(Boolean)
    .join("\n");
  ctx.toast(info, "info");
  if (ctx.rightSidebarCollapsed) ctx.openRightSidebar("drones");
}

// ── Registry ────────────────────────────────────────────────────────────────

const COMMANDS: SlashCommandDef[] = [
  { name: "new", execute: handleNew },
  { name: "clear", execute: handleClear },
  { name: "compact", execute: handleCompact },
  { name: "model", aliases: ["m"], execute: handleModel },
  { name: "steps", execute: handleSteps },
  { name: "undo", execute: handleUndo },
  { name: "sessions", execute: handleSessions },
  { name: "help", execute: handleHelp },
  { name: "launch", execute: handleLaunch },
  { name: "status", execute: handleStatus },
  { name: "stop", execute: handleStop },
  { name: "logs", aliases: ["log"], execute: handleLogs },
];

/**
 * Execute a slash command if the message starts with `/`.
 * Returns `true` if a command was matched and handled, `false` otherwise.
 */
export async function executeSlashCommand(
  message: string,
  ctx: SlashCommandContext,
): Promise<boolean> {
  const parts = message.slice(1).split(/\s+/);
  const cmdName = parts[0]?.toLowerCase();
  if (!cmdName) return false;

  const args = parts.slice(1);
  const command = COMMANDS.find((c) => c.name === cmdName || c.aliases?.includes(cmdName));
  if (!command) return false;

  await command.execute(args, ctx);
  return true;
}

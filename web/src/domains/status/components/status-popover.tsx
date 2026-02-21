import * as Popover from "@radix-ui/react-popover";
import { deriveHealth, useStatusQuery } from "../queries";
import type { DroneStatusBrief, McpServerInfo, OverallHealth, SystemStatus } from "../types";
import { StatusButton } from "./status-button";
import "./status-popover.css";

interface StatusPopoverProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function StatusPopover({ open, onOpenChange }: StatusPopoverProps) {
  const { data: status } = useStatusQuery(open);
  const health = deriveHealth(status);

  return (
    <Popover.Root open={open} onOpenChange={onOpenChange}>
      <Popover.Trigger asChild>
        <StatusButton health={health} />
      </Popover.Trigger>

      <Popover.Portal>
        <Popover.Content
          data-component="status-popover"
          side="right"
          sideOffset={8}
          align="end"
          collisionPadding={12}
        >
          {status ? <StatusContent status={status} health={health} /> : <LoadingState />}
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}

// ── Content ──────────────────────────────────────────────────────────────────

function StatusContent({ status, health }: { status: SystemStatus; health: OverallHealth }) {
  return (
    <>
      <div data-slot="status-header">
        <span data-slot="status-health-dot" data-health={health} />
        <span data-slot="status-header-title">System Status</span>
        <span data-slot="status-version">v{status.version}</span>
      </div>

      <SessionSection auth={status.auth} session={status.session} />
      <McpSection servers={status.mcp_servers} />
      <DronesSection drones={status.drones} />
    </>
  );
}

function LoadingState() {
  return <div data-slot="status-loading">Loading...</div>;
}

// ── Session / Auth Section ───────────────────────────────────────────────────

function SessionSection({
  auth,
  session,
}: {
  auth: SystemStatus["auth"];
  session: SystemStatus["session"];
}) {
  return (
    <div data-slot="status-section">
      <div data-slot="status-section-title">Session</div>
      <div data-slot="status-row">
        <span data-slot="status-label">Auth</span>
        <span data-slot="status-value">
          {auth.configured ? (
            <>
              <AuthBadge type={auth.auth_type} />
              {auth.expired && <span data-slot="status-stuck-badge">Expired</span>}
            </>
          ) : (
            <span data-slot="status-stuck-badge">Not configured</span>
          )}
        </span>
      </div>
      <div data-slot="status-row">
        <span data-slot="status-label">Active sessions</span>
        <span data-slot="status-value">
          {session.active_count} / {session.total_count}
        </span>
      </div>
    </div>
  );
}

function AuthBadge({ type }: { type: string | null }) {
  const label =
    type === "api_key" ? "API Key" : type === "oauth" ? "OAuth" : type === "bedrock" ? "Bedrock" : "Unknown";
  return <span data-slot="status-badge">{label}</span>;
}

// ── MCP Section ──────────────────────────────────────────────────────────────

function McpSection({ servers }: { servers: McpServerInfo[] }) {
  return (
    <div data-slot="status-section">
      <div data-slot="status-section-title">MCP Servers</div>
      {servers.length === 0 ? (
        <div data-slot="status-empty">No MCP servers configured</div>
      ) : (
        servers.map((server) => (
          <div key={server.name} data-slot="status-row">
            <span data-slot="status-label">{server.name}</span>
            <span data-slot="status-value" data-slot-detail="mcp-command">
              {server.command}
            </span>
          </div>
        ))
      )}
    </div>
  );
}

// ── Drones Section ───────────────────────────────────────────────────────────

function DronesSection({ drones }: { drones: DroneStatusBrief[] }) {
  return (
    <div data-slot="status-section">
      <div data-slot="status-section-title">Drones</div>
      {drones.length === 0 ? (
        <div data-slot="status-empty">No active drones</div>
      ) : (
        drones.map((drone) => <DroneRow key={drone.name} drone={drone} />)
      )}
    </div>
  );
}

function DroneRow({ drone }: { drone: DroneStatusBrief }) {
  const [done, total] = drone.progress;
  return (
    <div data-slot="status-row" data-slot-variant="drone">
      <span data-slot="status-health-dot" data-health={droneHealth(drone)} />
      <span data-slot="status-label">{drone.name}</span>
      <span data-slot="status-value">
        {done}/{total}
        {drone.is_stuck && <span data-slot="status-stuck-badge">Stuck</span>}
      </span>
      <span data-slot="status-meta">{drone.elapsed}</span>
    </div>
  );
}

function droneHealth(drone: DroneStatusBrief): OverallHealth {
  if (drone.is_stuck) return "warning";
  if (drone.liveness === "working") return "healthy";
  if (drone.liveness === "stopped" || drone.liveness === "crashed") return "error";
  return "unknown";
}

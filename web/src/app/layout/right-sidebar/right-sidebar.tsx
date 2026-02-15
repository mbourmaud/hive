import * as Tabs from "@radix-ui/react-tabs";
import { Bot, FileText, GitBranch, Info, PanelRightClose } from "lucide-react";
import type { ChatSession, ChatTurn, ContextUsage } from "@/domains/chat/types";
import type { RightSidebarTab } from "@/domains/monitor/store";
import type { DroneInfo } from "@/domains/monitor/types";
import { useResizablePanel } from "@/shared/hooks/use-resizable-panel";
import { ContextContent } from "./context-content";
import { DroneContent } from "./drone-content";
import { GitContent } from "./git-content";
import { PlansContent } from "./plans-content";
import "./right-sidebar.css";

// ── Constants ────────────────────────────────────────────────────────────────

const PANEL_MIN = 280;
const PANEL_MAX = 500;
const PANEL_DEFAULT = 320;
const COLLAPSE_THRESHOLD = 200;

// ── Types ────────────────────────────────────────────────────────────────────

interface RightSidebarProps {
  drones: DroneInfo[];
  connectionStatus: "connected" | "disconnected" | "mock";
  turns: ChatTurn[];
  contextUsage: ContextUsage | null;
  session: ChatSession | null;
  selectedModel?: string;
  activeTab: RightSidebarTab;
  collapsed: boolean;
  onTabChange: (tab: RightSidebarTab) => void;
  onToggleCollapse: () => void;
  onOpen: (tab: RightSidebarTab) => void;
}

// ── Tab config ──────────────────────────────────────────────────────────────

const TAB_CONFIG: { value: RightSidebarTab; label: string; icon: typeof Bot }[] = [
  { value: "drones", label: "Drones", icon: Bot },
  { value: "plans", label: "Plans", icon: FileText },
  { value: "git", label: "Git", icon: GitBranch },
  { value: "context", label: "Context", icon: Info },
];

// ── Component ────────────────────────────────────────────────────────────────

export function RightSidebar({
  drones,
  connectionStatus,
  turns,
  contextUsage,
  session,
  selectedModel,
  activeTab,
  collapsed: externalCollapsed,
  onTabChange,
  onToggleCollapse,
  onOpen,
}: RightSidebarProps) {
  const {
    width,
    collapsed: resizeCollapsed,
    onMouseDown,
  } = useResizablePanel({
    minWidth: PANEL_MIN,
    maxWidth: PANEL_MAX,
    defaultWidth: PANEL_DEFAULT,
    collapseThreshold: COLLAPSE_THRESHOLD,
    side: "right",
  });

  const isCollapsed = externalCollapsed || resizeCollapsed;

  if (isCollapsed) {
    return (
      <div data-slot="sidebar-collapsed-bar">
        {TAB_CONFIG.map(({ value, label, icon: Icon }) => (
          <button
            key={value}
            type="button"
            data-slot="sidebar-collapsed-btn"
            data-active={activeTab === value ? "" : undefined}
            onClick={() => onOpen(value)}
            title={`Show ${label}`}
          >
            <Icon className="h-4 w-4" />
          </button>
        ))}
      </div>
    );
  }

  return (
    <>
      {/* Drag handle */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: resize drag handle */}
      <div data-slot="sidebar-drag-handle" onMouseDown={onMouseDown} />

      <Tabs.Root
        data-component="right-sidebar"
        style={{ width: `${width}px` }}
        value={activeTab}
        onValueChange={(v) => onTabChange(v as RightSidebarTab)}
      >
        {/* Tab list header */}
        <Tabs.List data-slot="sidebar-tab-list">
          {TAB_CONFIG.map(({ value, label, icon: Icon }) => (
            <Tabs.Trigger key={value} value={value} data-slot="sidebar-tab-trigger">
              <Icon className="h-3.5 w-3.5" />
              {label}
              {value === "drones" && drones.length > 0 && (
                <span className="inline-flex items-center justify-center h-4 min-w-[16px] px-1 rounded-full bg-accent/15 text-accent text-[10px] font-bold leading-none">
                  {drones.length}
                </span>
              )}
            </Tabs.Trigger>
          ))}
          <button
            type="button"
            data-slot="sidebar-collapse-btn"
            onClick={onToggleCollapse}
            title="Hide sidebar"
          >
            <PanelRightClose className="h-3.5 w-3.5" />
          </button>
        </Tabs.List>

        {/* Drones tab */}
        <Tabs.Content value="drones" data-slot="sidebar-tab-content" className="flex flex-col">
          <DroneContent drones={drones} connectionStatus={connectionStatus} />
        </Tabs.Content>

        {/* Plans tab */}
        <Tabs.Content value="plans" data-slot="sidebar-tab-content" className="flex flex-col">
          <PlansContent onDispatch={() => onTabChange("drones")} />
        </Tabs.Content>

        {/* Git tab */}
        <Tabs.Content value="git" data-slot="sidebar-tab-content" className="flex flex-col">
          <GitContent />
        </Tabs.Content>

        {/* Context tab */}
        <Tabs.Content value="context" data-slot="sidebar-tab-content">
          <ContextContent
            turns={turns}
            contextUsage={contextUsage}
            session={session}
            selectedModel={selectedModel}
          />
        </Tabs.Content>
      </Tabs.Root>
    </>
  );
}

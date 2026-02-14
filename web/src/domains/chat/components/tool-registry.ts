import type { ComponentType } from "react";

export type ToolStatus = "pending" | "running" | "completed" | "error";

export interface ToolProps {
  input: Record<string, unknown>;
  output?: string;
  status: ToolStatus;
  hideDetails?: boolean;
  defaultOpen?: boolean;
  forceOpen?: boolean;
  locked?: boolean;
}

type ToolComponent = ComponentType<ToolProps>;
const TOOL_REGISTRY: Record<string, ToolComponent> = {};

export function registerTool(name: string, component: ToolComponent): void {
  TOOL_REGISTRY[name] = component;
}

export function getToolComponent(name: string): ToolComponent | undefined {
  return TOOL_REGISTRY[name];
}

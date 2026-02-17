// Import all tool renderers — each self-registers via registerTool()
import "./read-tool";
import "./edit-tool";
import "./write-tool";
import "./bash-tool";
import "./grep-tool";
import "./glob-tool";
import "./task-tool";
import "./web-fetch-tool";
import "./web-search-tool";
import "./todo-write-tool";
import "./session-search-tool";
import "./tool-search-tool";

export type { ToolProps } from "../tool-registry";
export { getToolComponent, registerTool } from "../tool-registry";
// Re-export the GenericTool fallback and registry helpers
export { GenericTool } from "./generic-tool";

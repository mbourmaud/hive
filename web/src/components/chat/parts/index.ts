// Import all tool renderers — each self-registers via registerTool()
import "./read-tool"
import "./edit-tool"
import "./write-tool"
import "./bash-tool"
import "./grep-tool"
import "./glob-tool"
import "./task-tool"
import "./web-fetch-tool"
import "./web-search-tool"
import "./todo-write-tool"

// Re-export the GenericTool fallback and registry helpers
export { GenericTool } from "./generic-tool"
export { registerTool, getToolComponent } from "../tool-registry"
export type { ToolProps } from "../tool-registry"

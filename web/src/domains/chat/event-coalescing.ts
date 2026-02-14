import type { StreamEvent } from "./types";

const STREAM_EVENT_TYPES: ReadonlySet<string> = new Set([
  "system",
  "assistant",
  "user",
  "result",
  "usage",
  "compact.completed",
]);

export function isStreamEvent(data: unknown): data is StreamEvent {
  if (typeof data !== "object" || data === null) return false;
  if (!("type" in data)) return false;
  const { type } = data;
  return typeof type === "string" && STREAM_EVENT_TYPES.has(type);
}

export function coalesceEvents(events: StreamEvent[]): StreamEvent[] {
  if (events.length <= 1) return events;

  const result: StreamEvent[] = [];
  let pendingText = "";

  for (const event of events) {
    if (
      event.type === "assistant" &&
      event.message.content.length === 1 &&
      event.message.content[0]?.type === "text"
    ) {
      pendingText += event.message.content[0].text;
    } else {
      if (pendingText) {
        result.push({
          type: "assistant",
          message: { content: [{ type: "text", text: pendingText }] },
        });
        pendingText = "";
      }
      result.push(event);
    }
  }

  if (pendingText) {
    result.push({
      type: "assistant",
      message: { content: [{ type: "text", text: pendingText }] },
    });
  }

  return result;
}

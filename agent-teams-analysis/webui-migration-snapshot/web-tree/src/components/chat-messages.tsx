import type { MessageInfo, MemberInfo } from "@/types/api";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { agentColor, agentInitials, agentColorIndex } from "./constants";

interface ChatMessagesProps {
  messages: MessageInfo[];
  members: MemberInfo[];
  leadModel: string | null;
}

export function ChatMessages({ messages, members }: ChatMessagesProps) {
  if (!messages.length) {
    return (
      <div className="text-sm italic py-3" style={{ color: "var(--text-muted)" }}>
        No messages exchanged yet.
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2 max-h-[400px] overflow-y-auto py-1">
      {messages.map((msg, i) => {
        const fromIdx = agentColorIndex(msg.from, members);
        const toIdx = agentColorIndex(msg.to, members);
        const fromColor = agentColor(fromIdx);
        const toColor = agentColor(toIdx);
        const initials = agentInitials(msg.from);
        const time = msg.timestamp
          ? new Date(msg.timestamp).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
          : "";

        return (
          <div key={i} className="flex gap-2.5 items-start py-2">
            <Avatar className="h-7 w-7 shrink-0">
              <AvatarFallback
                className="text-[11px] font-bold text-white"
                style={{ background: fromColor }}
              >
                {initials}
              </AvatarFallback>
            </Avatar>
            <div className="flex-1 min-w-0">
              <div className="flex items-baseline gap-2 mb-0.5">
                <span className="text-[13px] font-semibold" style={{ color: fromColor }}>
                  @{msg.from}
                </span>
                <span className="text-[11px]" style={{ color: "var(--text-muted)" }}>
                  &rarr;
                </span>
                <span className="text-xs font-medium" style={{ color: toColor }}>
                  @{msg.to}
                </span>
                <span className="text-[11px] ml-auto" style={{ color: "var(--text-muted)" }}>
                  {time}
                </span>
              </div>
              <div
                className="text-[13px] leading-relaxed p-2 px-3 rounded-b-lg rounded-tr-lg"
                style={{
                  color: "var(--text)",
                  background: "var(--surface2)",
                  border: "1px solid var(--border)",
                }}
              >
                {msg.text}
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}

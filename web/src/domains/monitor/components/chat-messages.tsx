import { ChevronRight } from "lucide-react";
import type { MemberInfo, MessageInfo } from "@/domains/monitor/types";
import { agentColor, agentColorIndex, agentInitials } from "@/shared/constants";
import { Avatar, AvatarFallback } from "@/shared/ui/avatar";

interface ChatMessagesProps {
  messages: MessageInfo[];
  members: MemberInfo[];
  leadModel: string | null;
}

export function ChatMessages({ messages, members }: ChatMessagesProps) {
  if (!messages || !messages.length) {
    return (
      <div className="text-sm italic py-3 text-muted-foreground">No messages exchanged yet.</div>
    );
  }

  return (
    <div className="flex flex-col gap-2 max-h-[400px] overflow-y-auto py-1">
      {messages.map((msg) => {
        const fromIdx = agentColorIndex(msg.from, members);
        const toIdx = agentColorIndex(msg.to, members);
        const fromColor = agentColor(fromIdx);
        const toColor = agentColor(toIdx);
        const initials = agentInitials(msg.from);
        const time = msg.timestamp
          ? new Date(msg.timestamp).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
          : "";

        return (
          <div
            key={`${msg.from}-${msg.to}-${msg.timestamp}`}
            className="flex gap-2.5 items-start py-2"
          >
            <Avatar className="h-6 w-6 shrink-0">
              <AvatarFallback
                className="text-[10px] font-bold text-white"
                style={{ background: fromColor }}
              >
                {initials}
              </AvatarFallback>
            </Avatar>
            <div className="flex-1 min-w-0">
              <div className="flex items-baseline gap-2 mb-0.5">
                <span className="text-[13px] font-bold" style={{ color: fromColor }}>
                  @{msg.from}
                </span>
                <ChevronRight className="w-3 h-3 text-muted-foreground shrink-0" />
                <span className="text-xs font-medium" style={{ color: toColor }}>
                  @{msg.to}
                </span>
                <span className="text-[11px] ml-auto text-muted-foreground">{time}</span>
              </div>
              <div
                className="text-[13px] leading-relaxed p-2 px-3 rounded-lg text-foreground bg-muted border border-border border-l-2"
                style={{ borderLeftColor: fromColor }}
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

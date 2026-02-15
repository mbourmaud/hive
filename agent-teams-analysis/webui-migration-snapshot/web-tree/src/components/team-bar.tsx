import type { MemberInfo } from "@/types/api";
import { agentColor, shortModel } from "./constants";

interface TeamBarProps {
  members: MemberInfo[];
  leadModel: string | null;
  droneLiveness: string;
}

function livenessClass(liveness: string) {
  switch (liveness) {
    case "working": return "bg-[var(--green)] shadow-[0_0_5px_rgba(34,197,94,0.4)]";
    case "idle": return "bg-[var(--yellow)]";
    case "completed": return "bg-[var(--accent)]";
    case "dead": return "bg-[var(--red)]";
    default: return "bg-[var(--text-muted)]";
  }
}

export function TeamBar({ members, leadModel, droneLiveness }: TeamBarProps) {
  if (members.length === 0) return null;

  return (
    <div className="flex items-center gap-2 flex-wrap">
      <span className="text-xs font-semibold uppercase tracking-wider" style={{ color: "var(--text-muted)" }}>
        Team
      </span>

      {/* Lead tag */}
      <span
        className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-[13px] font-semibold"
        style={{
          background: "var(--surface2)",
          border: "1px solid var(--border)",
        }}
      >
        <span className={`w-[7px] h-[7px] rounded-full shrink-0 ${livenessClass(droneLiveness)}`} />
        <span style={{ color: agentColor(0) }}>@lead</span>
        <span className="text-[11px] font-normal" style={{ color: "var(--text-muted)" }}>
          ({shortModel(leadModel || "?")})
        </span>
      </span>

      <span className="text-xs" style={{ color: "var(--text-muted)" }}>|</span>

      {/* Member tags */}
      {members.map((member, i) => (
        <span
          key={member.name}
          className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-[13px] font-semibold"
          style={{
            background: "var(--surface2)",
            border: "1px solid var(--border)",
          }}
        >
          <span className={`w-[7px] h-[7px] rounded-full shrink-0 ${livenessClass(member.liveness)}`} />
          <span style={{ color: agentColor(i + 1) }}>@{member.name}</span>
          <span className="text-[11px] font-normal" style={{ color: "var(--text-muted)" }}>
            ({shortModel(member.model)})
          </span>
        </span>
      ))}
    </div>
  );
}

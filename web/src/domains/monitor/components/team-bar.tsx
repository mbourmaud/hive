import type { MemberInfo } from "@/domains/monitor/types";
import { agentColor, shortModel } from "@/shared/constants";

interface TeamBarProps {
  members: MemberInfo[];
  leadModel: string | null;
  droneLiveness: string;
}

function livenessRing(liveness: string) {
  switch (liveness) {
    case "working":
      return "ring-2 ring-white/30 ring-offset-1 ring-offset-card";
    case "completed":
      return "ring-2 ring-white/30 ring-offset-1 ring-offset-card";
    case "dead":
      return "ring-2 ring-destructive/40 ring-offset-1 ring-offset-card";
    default:
      return "";
  }
}

function PulseDot({ isWorking }: { isWorking: boolean }) {
  if (!isWorking) return null;
  return (
    <span className="relative flex h-2 w-2 ml-1">
      <span className="absolute inline-flex h-full w-full rounded-full bg-honey animate-[pulse-ring_2s_ease-out_infinite]" />
      <span className="relative inline-flex rounded-full h-2 w-2 bg-honey" />
    </span>
  );
}

export function TeamBar({ members, leadModel, droneLiveness }: TeamBarProps) {
  if (members.length === 0) return null;

  return (
    <div className="flex items-center gap-2 flex-wrap">
      {/* Lead */}
      <span
        className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[11px] font-semibold text-white ${livenessRing(droneLiveness)}`}
        style={{ backgroundColor: agentColor(0) }}
      >
        lead
        <span className="text-[10px] font-normal text-white/60">
          {shortModel(leadModel || "?")}
        </span>
        <PulseDot isWorking={droneLiveness === "working"} />
      </span>

      {/* Workers */}
      {members.map((member, i) => (
        <span
          key={member.name}
          className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[11px] font-semibold text-white ${livenessRing(member.liveness)}`}
          style={{ backgroundColor: agentColor(i + 1) }}
        >
          {member.name}
          <span className="text-[10px] font-normal text-white/60">{shortModel(member.model)}</span>
          <PulseDot isWorking={member.liveness === "working"} />
        </span>
      ))}
    </div>
  );
}

import type { CostInfo } from "@/types/api";
import { fmtCost, fmtTokens } from "./constants";

interface CostCardProps {
  cost: CostInfo;
}

export function CostCard({ cost }: CostCardProps) {
  return (
    <div className="flex gap-6 text-sm" style={{ color: "var(--text-muted)" }}>
      <span>
        Total:{" "}
        <strong className="font-bold" style={{ color: "var(--accent)" }}>
          {fmtCost(cost.total_usd)}
        </strong>
      </span>
      <span>
        In: <strong style={{ color: "var(--text)" }}>{fmtTokens(cost.input_tokens)}</strong>
      </span>
      <span>
        Out: <strong style={{ color: "var(--text)" }}>{fmtTokens(cost.output_tokens)}</strong>
      </span>
    </div>
  );
}

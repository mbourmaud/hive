import type { CostInfo } from "@/types/api";
import { fmtCost, fmtTokens } from "./constants";

interface CostCardProps {
  cost: CostInfo;
}

export function CostCard({ cost }: CostCardProps) {
  return (
    <div className="flex gap-6 text-sm text-muted-foreground">
      <span>Total: <strong className="font-bold text-accent">{fmtCost(cost.total_usd)}</strong></span>
      <span>In: <strong className="text-foreground">{fmtTokens(cost.input_tokens)}</strong></span>
      <span>Out: <strong className="text-foreground">{fmtTokens(cost.output_tokens)}</strong></span>
    </div>
  );
}

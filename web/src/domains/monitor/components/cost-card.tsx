import type { CostInfo } from "@/domains/monitor/types";
import { fmtCost, fmtTokens } from "@/shared/constants";

interface CostCardProps {
  cost: CostInfo;
}

export function CostCard({ cost }: CostCardProps) {
  const hasCache = cost.cache_creation_tokens > 0 || cost.cache_read_tokens > 0;

  return (
    <div className="flex gap-6 text-sm text-muted-foreground flex-wrap">
      <span>
        Total: <strong className="font-bold text-accent">{fmtCost(cost.total_usd)}</strong>
      </span>
      <span>
        In: <strong className="text-foreground">{fmtTokens(cost.input_tokens)}</strong>
      </span>
      <span>
        Out: <strong className="text-foreground">{fmtTokens(cost.output_tokens)}</strong>
      </span>
      {hasCache && (
        <>
          <span>
            Cache W: <strong className="text-foreground">{fmtTokens(cost.cache_creation_tokens)}</strong>
          </span>
          <span>
            Cache R: <strong className="text-foreground">{fmtTokens(cost.cache_read_tokens)}</strong>
          </span>
        </>
      )}
    </div>
  );
}

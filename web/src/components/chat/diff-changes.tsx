interface DiffChangesProps {
  additions: number
  deletions: number
  variant?: "default" | "bars"
}

export function DiffChanges({
  additions,
  deletions,
  variant = "default",
}: DiffChangesProps) {
  if (variant === "bars") {
    return <DiffBars additions={additions} deletions={deletions} />
  }

  return (
    <span data-component="diff-changes" className="inline-flex items-center gap-1.5 text-xs font-mono">
      {additions > 0 && (
        <span className="text-green-600 dark:text-green-400">+{additions}</span>
      )}
      {deletions > 0 && (
        <span className="text-red-600 dark:text-red-400">-{deletions}</span>
      )}
    </span>
  )
}

// 5 SVG blocks, proportionally colored green/red
function DiffBars({
  additions,
  deletions,
}: {
  additions: number
  deletions: number
}) {
  const total = additions + deletions
  if (total === 0) return null

  const BLOCK_COUNT = 5
  const greenBlocks = Math.round((additions / total) * BLOCK_COUNT)

  return (
    <svg
      data-component="diff-bars"
      width={BLOCK_COUNT * 6 - 4}
      height={12}
      viewBox={`0 0 ${BLOCK_COUNT * 6 - 4} 12`}
      className="inline-block align-middle"
    >
      {Array.from({ length: BLOCK_COUNT }, (_, i) => {
        const isGreen = i < greenBlocks
        return (
          <rect
            key={i}
            x={i * 6}
            y={0}
            width={2}
            height={12}
            rx={0.5}
            fill={isGreen ? "var(--color-green-500, #22c55e)" : "var(--color-red-500, #ef4444)"}
          />
        )
      })}
    </svg>
  )
}

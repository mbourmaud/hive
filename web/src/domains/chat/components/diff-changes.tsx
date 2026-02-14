import "./diff-changes.css";

interface DiffChangesProps {
  additions: number;
  deletions: number;
  variant?: "default" | "bars";
}

export function DiffChanges({
  additions,
  deletions,
  variant = "default",
}: DiffChangesProps) {
  if (variant === "bars") {
    const total = additions + deletions;
    const blocks = Math.min(total, 5);
    const additionBlocks = Math.round((additions / total) * blocks);

    return (
      <div className="diff-changes-bars">
        {Array.from({ length: blocks }).map((_, i) => (
          <div
            key={i}
            className={`diff-changes-bar ${i < additionBlocks ? "addition" : "deletion"}`}
          />
        ))}
      </div>
    );
  }

  return (
    <div className="diff-changes">
      {additions > 0 && (
        <span className="diff-changes-addition">+{additions}</span>
      )}
      {deletions > 0 && (
        <span className="diff-changes-deletion">-{deletions}</span>
      )}
    </div>
  );
}

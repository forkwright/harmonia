interface Props {
  score: number | null;
}

function tier(score: number): { label: string; className: string } {
  if (score >= 90) return { label: "Lossless", className: "bg-green-900 text-green-300" };
  if (score >= 70) return { label: "High", className: "bg-blue-900 text-blue-300" };
  if (score >= 50) return { label: "Good", className: "bg-yellow-900 text-yellow-300" };
  return { label: "Low", className: "bg-red-900 text-red-300" };
}

export default function QualityBadge({ score }: Props) {
  if (score === null) {
    return (
      <span className="px-2 py-0.5 text-xs font-medium rounded bg-gray-700 text-gray-400">
        Unknown
      </span>
    );
  }

  const { label, className } = tier(score);
  return (
    <span className={`px-2 py-0.5 text-xs font-medium rounded ${className}`} title={`Score: ${score}`}>
      {label}
    </span>
  );
}

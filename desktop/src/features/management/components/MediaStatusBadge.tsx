const STATUS_COLORS: Record<string, string> = {
  imported: "bg-blue-900 text-blue-300",
  enriched: "bg-purple-900 text-purple-300",
  organized: "bg-indigo-900 text-indigo-300",
  available: "bg-green-900 text-green-300",
  failed: "bg-red-900 text-red-300",
  missing: "bg-gray-700 text-gray-400",
};

interface Props {
  status: string;
}

export default function MediaStatusBadge({ status }: Props) {
  const colorClass = STATUS_COLORS[status] ?? "bg-gray-700 text-gray-400";
  return (
    <span className={`px-2 py-0.5 text-xs font-medium rounded ${colorClass}`}>{status}</span>
  );
}

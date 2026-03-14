import { useNavigate } from "react-router-dom";
import MediaStatusBadge from "./MediaStatusBadge";
import QualityBadge from "./QualityBadge";
import type { MediaItem } from "../../../types/management";

function formatBytes(bytes: number | null): string {
  if (bytes === null) return "—";
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

interface Props {
  item: MediaItem;
  selected: boolean;
  onSelect: (id: string, checked: boolean) => void;
}

export default function MediaItemRow({ item, selected, onSelect }: Props) {
  const navigate = useNavigate();

  return (
    <tr
      className="border-b border-gray-800 hover:bg-gray-800/50 cursor-pointer"
      onClick={() => navigate(`/manage/media/${item.id}`)}
    >
      <td className="px-3 py-2" onClick={(e) => e.stopPropagation()}>
        <input
          type="checkbox"
          checked={selected}
          onChange={(e) => onSelect(item.id, e.target.checked)}
          className="rounded border-gray-600 bg-gray-700"
        />
      </td>
      <td className="px-3 py-2 text-sm text-gray-100">{item.title}</td>
      <td className="px-3 py-2">
        <MediaStatusBadge status={item.status} />
      </td>
      <td className="px-3 py-2">
        <QualityBadge score={item.qualityScore} />
      </td>
      <td className="px-3 py-2 text-sm text-gray-400">{formatBytes(item.fileSize)}</td>
      <td className="px-3 py-2 text-sm text-gray-400">{formatDate(item.addedAt)}</td>
    </tr>
  );
}

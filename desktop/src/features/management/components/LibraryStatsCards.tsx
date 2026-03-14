import type { LibraryHealthReport, MediaType } from "../../../types/management";

const MEDIA_LABELS: Record<MediaType, string> = {
  music: "Music",
  audiobook: "Audiobooks",
  ebook: "Ebooks",
  podcast: "Podcasts",
  manga: "Manga",
  news: "News",
  movie: "Movies",
  tv: "TV",
};

interface Props {
  report: LibraryHealthReport;
}

export default function LibraryStatsCards({ report }: Props) {
  return (
    <div className="space-y-4">
      <div className="grid grid-cols-4 gap-3">
        <StatCard label="Total Items" value={report.totalItems.toLocaleString()} />
        <StatCard
          label="Missing Metadata"
          value={report.missingMetadata.toLocaleString()}
          highlight={report.missingMetadata > 0}
        />
        <StatCard
          label="Orphaned Files"
          value={report.orphanedFiles.toLocaleString()}
          highlight={report.orphanedFiles > 0}
        />
        <StatCard
          label="Duplicates"
          value={report.duplicates.toLocaleString()}
          highlight={report.duplicates > 0}
        />
      </div>
      <div className="grid grid-cols-4 gap-3">
        {(Object.entries(report.byMediaType) as [MediaType, number][]).map(([type, count]) => (
          <StatCard key={type} label={MEDIA_LABELS[type]} value={count.toLocaleString()} />
        ))}
      </div>
    </div>
  );
}

function StatCard({
  label,
  value,
  highlight,
}: {
  label: string;
  value: string;
  highlight?: boolean;
}) {
  return (
    <div
      className={`p-4 rounded border ${highlight ? "border-red-700 bg-red-900/20" : "border-gray-700 bg-gray-800/50"}`}
    >
      <p className="text-xs text-gray-400 mb-1">{label}</p>
      <p className={`text-2xl font-semibold ${highlight ? "text-red-300" : "text-gray-100"}`}>
        {value}
      </p>
    </div>
  );
}

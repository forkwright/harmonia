import { useLibraryHealth } from "../hooks/useLibraryHealth";
import { useScanStatus } from "../hooks/useScanStatus";
import LibraryStatsCards from "../components/LibraryStatsCards";
import ScanProgress from "../components/ScanProgress";
import type { QualityBucket } from "../../../types/management";

function QualityBar({ bucket, max }: { bucket: QualityBucket; max: number }) {
  const width = max > 0 ? (bucket.count / max) * 100 : 0;
  return (
    <div className="flex items-center gap-3">
      <span className="text-xs text-gray-400 w-32 truncate flex-shrink-0">{bucket.label}</span>
      <div className="flex-1 bg-gray-700 rounded h-4 relative">
        <div
          className="bg-blue-500 h-4 rounded transition-all"
          style={{ width: `${width}%` }}
        />
      </div>
      <span className="text-xs text-gray-300 w-16 text-right flex-shrink-0">
        {bucket.count.toLocaleString()} ({bucket.percentage.toFixed(1)}%)
      </span>
    </div>
  );
}

export default function LibraryHealthPage() {
  const health = useLibraryHealth();
  const { status: scan, trigger } = useScanStatus();

  if (health.isLoading) {
    return <div className="p-6 text-sm text-gray-500">Loading health report…</div>;
  }
  if (health.isError) {
    return <div className="p-6 text-sm text-red-400">Failed to load health report.</div>;
  }
  if (!health.data) {
    return null;
  }

  const report = health.data;
  const maxBucketCount = Math.max(...report.qualityDistribution.map((b) => b.count), 1);

  return (
    <div className="h-full overflow-y-auto p-6 space-y-6">
      <h1 className="text-xl font-semibold text-gray-100">Library Health</h1>

      {scan.data && (
        <ScanProgress
          status={scan.data}
          onTrigger={() => trigger.mutate(undefined)}
          triggering={trigger.isPending}
        />
      )}

      <section>
        <h2 className="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
          Overview
        </h2>
        <LibraryStatsCards report={report} />
      </section>

      {report.qualityDistribution.length > 0 && (
        <section>
          <h2 className="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
            Quality Distribution
          </h2>
          <div className="space-y-2">
            {report.qualityDistribution.map((bucket) => (
              <QualityBar key={bucket.label} bucket={bucket} max={maxBucketCount} />
            ))}
          </div>
        </section>
      )}

      <section className="grid grid-cols-3 gap-4">
        <HealthSection
          title="Missing Metadata"
          count={report.missingMetadata}
          description="Items without complete metadata."
          severity={report.missingMetadata > 0 ? "warn" : "ok"}
        />
        <HealthSection
          title="Orphaned Files"
          count={report.orphanedFiles}
          description="Files in library directories not tracked in the database."
          severity={report.orphanedFiles > 0 ? "warn" : "ok"}
        />
        <HealthSection
          title="Duplicates"
          count={report.duplicates}
          description="Potential duplicate items detected."
          severity={report.duplicates > 0 ? "warn" : "ok"}
        />
      </section>

      <section>
        <h2 className="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
          Status Breakdown
        </h2>
        <div className="grid grid-cols-3 gap-3">
          {Object.entries(report.byStatus).map(([status, count]) => (
            <div key={status} className="p-3 rounded border border-gray-700 bg-gray-800/50">
              <p className="text-xs text-gray-400 capitalize">{status}</p>
              <p className="text-lg font-semibold text-gray-100">{count.toLocaleString()}</p>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}

function HealthSection({
  title,
  count,
  description,
  severity,
}: {
  title: string;
  count: number;
  description: string;
  severity: "ok" | "warn";
}) {
  return (
    <div
      className={`p-4 rounded border ${severity === "warn" && count > 0 ? "border-yellow-700 bg-yellow-900/10" : "border-gray-700 bg-gray-800/50"}`}
    >
      <p className="text-sm font-medium text-gray-200 mb-1">{title}</p>
      <p
        className={`text-2xl font-semibold mb-1 ${severity === "warn" && count > 0 ? "text-yellow-300" : "text-gray-100"}`}
      >
        {count.toLocaleString()}
      </p>
      <p className="text-xs text-gray-500">{description}</p>
    </div>
  );
}

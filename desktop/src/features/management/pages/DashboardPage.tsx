import { useScanStatus } from "../hooks/useScanStatus";
import { useLibraryHealth } from "../hooks/useLibraryHealth";
import { useDownloadQueue } from "../hooks/useDownloadQueue";
import { useRequests } from "../hooks/useRequests";
import ScanProgress from "../components/ScanProgress";
import LibraryStatsCards from "../components/LibraryStatsCards";
import DownloadRow from "../components/DownloadRow";
import { Link } from "react-router-dom";

export default function DashboardPage() {
  const { status: scan, trigger } = useScanStatus();
  const health = useLibraryHealth();
  const { queue, cancel, retry } = useDownloadQueue();
  const { requests } = useRequests({ status: "pending" });

  const pendingCount = requests.data?.data.length ?? 0;
  const activeDownloads = queue.data?.active ?? [];

  return (
    <div className="h-full overflow-y-auto p-6 space-y-6">
      <h1 className="text-xl font-semibold text-gray-100">Management Dashboard</h1>

      {scan.data && (
        <ScanProgress
          status={scan.data}
          onTrigger={() => trigger.mutate(undefined)}
          triggering={trigger.isPending}
        />
      )}
      {scan.isLoading && <p className="text-sm text-gray-500">Loading scan status…</p>}

      <section>
        <div className="flex items-center justify-between mb-3">
          <h2 className="text-sm font-medium text-gray-300 uppercase tracking-wider">
            Library Overview
          </h2>
          <Link to="/manage/health" className="text-xs text-blue-400 hover:text-blue-300">
            Full health report →
          </Link>
        </div>
        {health.isLoading && <p className="text-sm text-gray-500">Loading health report…</p>}
        {health.isError && <p className="text-sm text-red-400">Failed to load health report.</p>}
        {health.data && <LibraryStatsCards report={health.data} />}
      </section>

      <section>
        <div className="flex items-center justify-between mb-3">
          <h2 className="text-sm font-medium text-gray-300 uppercase tracking-wider">
            Active Downloads
          </h2>
          <Link to="/manage/downloads" className="text-xs text-blue-400 hover:text-blue-300">
            View all →
          </Link>
        </div>
        {queue.isLoading && <p className="text-sm text-gray-500">Loading download queue…</p>}
        {activeDownloads.length === 0 && !queue.isLoading && (
          <p className="text-sm text-gray-500">No active downloads.</p>
        )}
        <div className="space-y-2">
          {activeDownloads.map((dl) => (
            <DownloadRow
              key={dl.id}
              download={dl}
              onCancel={(id) => cancel.mutate(id)}
              onRetry={(id) => retry.mutate(id)}
              cancelling={cancel.isPending}
              retrying={retry.isPending}
            />
          ))}
        </div>
      </section>

      {pendingCount > 0 && (
        <section>
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-sm font-medium text-gray-300 uppercase tracking-wider">
              Pending Requests
            </h2>
            <Link to="/manage/requests" className="text-xs text-blue-400 hover:text-blue-300">
              Review {pendingCount} →
            </Link>
          </div>
          <p className="text-sm text-yellow-400">
            {pendingCount} request{pendingCount !== 1 ? "s" : ""} awaiting decision.
          </p>
        </section>
      )}
    </div>
  );
}

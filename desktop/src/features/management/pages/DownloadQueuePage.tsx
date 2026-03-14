import { useDownloadQueue } from "../hooks/useDownloadQueue";
import DownloadRow from "../components/DownloadRow";

function Section({ title, count }: { title: string; count: number }) {
  return (
    <h2 className="text-sm font-medium text-gray-400 uppercase tracking-wider">
      {title}
      <span className="ml-2 text-gray-600">({count})</span>
    </h2>
  );
}

export default function DownloadQueuePage() {
  const { queue, cancel, retry } = useDownloadQueue();

  if (queue.isLoading) {
    return <div className="p-6 text-sm text-gray-500">Loading…</div>;
  }
  if (queue.isError) {
    return <div className="p-6 text-sm text-red-400">Failed to load download queue.</div>;
  }

  const { active = [], queued = [], completed = [], failed = [] } = queue.data ?? {};

  return (
    <div className="h-full overflow-y-auto p-6 space-y-6">
      <h1 className="text-xl font-semibold text-gray-100">Download Queue</h1>

      <section className="space-y-2">
        <Section title="Active" count={active.length} />
        {active.length === 0 ? (
          <p className="text-sm text-gray-500">Nothing downloading.</p>
        ) : (
          <div className="space-y-2">
            {active.map((dl) => (
              <DownloadRow
                key={dl.id}
                download={dl}
                onCancel={(id) => cancel.mutate(id)}
                cancelling={cancel.isPending}
              />
            ))}
          </div>
        )}
      </section>

      <section className="space-y-2">
        <Section title="Queued" count={queued.length} />
        {queued.length === 0 ? (
          <p className="text-sm text-gray-500">Queue empty.</p>
        ) : (
          <div className="space-y-2">
            {queued.map((dl) => (
              <DownloadRow
                key={dl.id}
                download={dl}
                onCancel={(id) => cancel.mutate(id)}
                cancelling={cancel.isPending}
              />
            ))}
          </div>
        )}
      </section>

      <section className="space-y-2">
        <Section title="Failed" count={failed.length} />
        {failed.length === 0 ? (
          <p className="text-sm text-gray-500">No failed downloads.</p>
        ) : (
          <div className="space-y-2">
            {failed.map((dl) => (
              <DownloadRow
                key={dl.id}
                download={dl}
                onRetry={(id) => retry.mutate(id)}
                retrying={retry.isPending}
              />
            ))}
          </div>
        )}
      </section>

      <section className="space-y-2">
        <Section title="Completed" count={completed.length} />
        {completed.length === 0 ? (
          <p className="text-sm text-gray-500">No completed downloads.</p>
        ) : (
          <div className="space-y-2">
            {completed.map((dl) => (
              <DownloadRow key={dl.id} download={dl} />
            ))}
          </div>
        )}
      </section>
    </div>
  );
}

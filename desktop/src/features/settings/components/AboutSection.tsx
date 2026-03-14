import type { AppInfo } from "../hooks/useSettings";

interface Props {
  appInfo: AppInfo | null;
}

export default function AboutSection({ appInfo }: Props) {
  return (
    <section className="space-y-4">
      <h2 className="text-base font-semibold text-gray-100">About</h2>

      <div className="bg-gray-900 rounded-lg px-4 py-3 space-y-1.5 text-sm">
        {appInfo ? (
          <>
            <Row label="Version" value={appInfo.version} />
            <Row label="Tauri" value={appInfo.tauri_version} />
            <Row label="Platform" value={`${appInfo.os} / ${appInfo.arch}`} />
          </>
        ) : (
          <p className="text-gray-500">Loading…</p>
        )}
      </div>
    </section>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between">
      <span className="text-gray-400">{label}</span>
      <span className="text-gray-200 font-mono text-xs">{value}</span>
    </div>
  );
}

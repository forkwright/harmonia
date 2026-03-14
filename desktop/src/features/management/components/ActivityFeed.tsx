import type { MediaEvent } from "../../../types/management";

function formatTime(iso: string): string {
  return new Date(iso).toLocaleTimeString(undefined, {
    hour: "2-digit",
    minute: "2-digit",
  });
}

interface Props {
  events: MediaEvent[];
}

export default function ActivityFeed({ events }: Props) {
  if (events.length === 0) {
    return <p className="text-sm text-gray-500 py-4">No recent activity.</p>;
  }

  return (
    <ul className="space-y-1.5">
      {events.map((event, i) => (
        <li key={i} className="flex gap-3 text-sm">
          <span className="text-gray-500 flex-shrink-0 w-16">{formatTime(event.timestamp)}</span>
          <span className="text-gray-400 flex-shrink-0 w-24 capitalize">{event.type}</span>
          <span className="text-gray-300 truncate">{event.detail}</span>
        </li>
      ))}
    </ul>
  );
}

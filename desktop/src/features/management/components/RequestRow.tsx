import { useState } from "react";
import type { MediaRequest } from "../../../types/management";

const STATUS_COLORS: Record<MediaRequest["status"], string> = {
  pending: "bg-yellow-900 text-yellow-300",
  approved: "bg-green-900 text-green-300",
  denied: "bg-red-900 text-red-300",
  fulfilled: "bg-blue-900 text-blue-300",
};

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

interface Props {
  request: MediaRequest;
  isAdmin: boolean;
  onApprove?: (id: string) => void;
  onDeny?: (id: string, reason: string) => void;
  onCancel?: (id: string) => void;
}

export default function RequestRow({ request, isAdmin, onApprove, onDeny, onCancel }: Props) {
  const [denyReason, setDenyReason] = useState("");
  const [showDenyForm, setShowDenyForm] = useState(false);

  function handleDeny() {
    if (!denyReason.trim()) return;
    onDeny?.(request.id, denyReason);
    setShowDenyForm(false);
    setDenyReason("");
  }

  return (
    <div className="p-3 rounded bg-gray-800/50 border border-gray-700 space-y-2">
      <div className="flex items-center justify-between gap-4">
        <div className="flex-1 min-w-0">
          <p className="text-sm text-gray-100 truncate">{request.title}</p>
          <p className="text-xs text-gray-400">
            {request.userName} · {request.mediaType} · {formatDate(request.createdAt)}
          </p>
        </div>
        <span className={`px-2 py-0.5 text-xs font-medium rounded ${STATUS_COLORS[request.status]}`}>
          {request.status}
        </span>
      </div>
      {request.denyReason && (
        <p className="text-xs text-red-400">Reason: {request.denyReason}</p>
      )}
      {isAdmin && request.status === "pending" && (
        <div className="flex gap-2">
          {!showDenyForm && (
            <>
              <button
                onClick={() => onApprove?.(request.id)}
                className="px-3 py-1 text-xs rounded bg-green-700 hover:bg-green-600 text-white transition-colors"
              >
                Approve
              </button>
              <button
                onClick={() => setShowDenyForm(true)}
                className="px-3 py-1 text-xs rounded bg-red-800 hover:bg-red-700 text-white transition-colors"
              >
                Deny
              </button>
              <button
                onClick={() => onCancel?.(request.id)}
                className="px-3 py-1 text-xs rounded bg-gray-700 hover:bg-gray-600 text-gray-300 transition-colors"
              >
                Cancel
              </button>
            </>
          )}
          {showDenyForm && (
            <div className="flex gap-2 flex-1">
              <input
                type="text"
                value={denyReason}
                onChange={(e) => setDenyReason(e.target.value)}
                placeholder="Reason for denial"
                className="flex-1 px-2 py-1 text-xs rounded bg-gray-700 border border-gray-600 text-gray-100 focus:outline-none focus:border-blue-500"
              />
              <button
                onClick={handleDeny}
                className="px-3 py-1 text-xs rounded bg-red-700 hover:bg-red-600 text-white transition-colors"
              >
                Confirm
              </button>
              <button
                onClick={() => setShowDenyForm(false)}
                className="px-3 py-1 text-xs rounded bg-gray-700 hover:bg-gray-600 text-gray-300 transition-colors"
              >
                Cancel
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

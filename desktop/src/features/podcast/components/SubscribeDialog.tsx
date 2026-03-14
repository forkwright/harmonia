/** RSS feed URL input dialog for adding a new podcast subscription. */

import { useState } from "react";
import { useSubscribe } from "../hooks/useSubscriptions";

interface Props {
  open: boolean;
  onClose: () => void;
}

function isValidUrl(value: string): boolean {
  try {
    const url = new URL(value);
    return url.protocol === "http:" || url.protocol === "https:";
  } catch {
    return false;
  }
}

export default function SubscribeDialog({ open, onClose }: Props) {
  const [feedUrl, setFeedUrl] = useState("");
  const [urlError, setUrlError] = useState<string | null>(null);
  const subscribe = useSubscribe();

  if (!open) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setUrlError(null);

    if (!isValidUrl(feedUrl)) {
      setUrlError("Enter a valid http:// or https:// URL.");
      return;
    }

    subscribe.mutate(feedUrl, {
      onSuccess: () => {
        setFeedUrl("");
        onClose();
      },
    });
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
      role="dialog"
      aria-modal="true"
      aria-labelledby="subscribe-dialog-title"
    >
      <div className="bg-gray-900 rounded-xl shadow-2xl w-full max-w-md mx-4 p-6">
        <h2 id="subscribe-dialog-title" className="text-lg font-semibold text-gray-100 mb-4">
          Subscribe to Podcast
        </h2>
        <form onSubmit={handleSubmit} noValidate>
          <label htmlFor="feed-url" className="block text-sm text-gray-400 mb-1">
            RSS feed URL
          </label>
          <input
            id="feed-url"
            type="url"
            value={feedUrl}
            onChange={(e) => setFeedUrl(e.target.value)}
            placeholder="https://example.com/feed.xml"
            className="w-full px-3 py-2 rounded-lg bg-gray-800 border border-gray-700 text-gray-100 placeholder-gray-500 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
            autoFocus
          />
          {urlError && <p className="mt-1 text-xs text-red-400">{urlError}</p>}
          {subscribe.isError && (
            <p className="mt-1 text-xs text-red-400">Failed to subscribe. Check the URL and try again.</p>
          )}
          <div className="flex justify-end gap-3 mt-5">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 rounded-lg text-sm text-gray-400 hover:text-gray-200 transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={subscribe.isPending || feedUrl.length === 0}
              className="px-4 py-2 rounded-lg bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed text-white text-sm font-medium transition-colors"
            >
              {subscribe.isPending ? "Subscribing…" : "Subscribe"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

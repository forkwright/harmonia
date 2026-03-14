interface Props {
  onBookmark: () => void;
  label?: string;
}

export default function BookmarkButton({ onBookmark, label = "Bookmark" }: Props) {
  return (
    <button
      type="button"
      onClick={onBookmark}
      className="flex items-center gap-1.5 px-3 py-1.5 bg-gray-700 text-gray-300 hover:bg-gray-600 hover:text-white rounded transition-colors text-sm"
      title="Add bookmark at current position"
    >
      <svg className="w-4 h-4" viewBox="0 0 24 24" fill="currentColor">
        <path d="M17 3H7c-1.1 0-2 .9-2 2v16l7-3 7 3V5c0-1.1-.9-2-2-2z" />
      </svg>
      {label}
    </button>
  );
}

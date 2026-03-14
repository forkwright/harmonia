interface Props {
  chapterIndex: number;
  totalChapters: number;
  percentComplete: number;
  chapterTitle: string;
}

export default function ProgressIndicator({
  chapterIndex,
  totalChapters,
  percentComplete,
  chapterTitle,
}: Props) {
  return (
    <div className="space-y-1">
      <div className="flex justify-between text-xs text-gray-400">
        <span>
          Chapter {chapterIndex + 1} of {totalChapters}
        </span>
        <span>{Math.round(percentComplete)}% complete</span>
      </div>
      <div className="text-xs text-gray-500 truncate">{chapterTitle}</div>
      <div className="w-full bg-gray-700 rounded-full h-1.5">
        <div
          className="bg-blue-500 h-1.5 rounded-full transition-all"
          style={{ width: `${Math.min(100, percentComplete)}%` }}
        />
      </div>
    </div>
  );
}

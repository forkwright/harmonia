import { useEffect, useRef } from "react";
import ChapterRow from "./ChapterRow";
import type { Chapter } from "../../../types/media";

interface Props {
  chapters: Chapter[];
  currentChapterPosition: number;
  onSelect: (chapter: Chapter) => void;
}

export default function ChapterList({ chapters, currentChapterPosition, onSelect }: Props) {
  const listRef = useRef<HTMLDivElement>(null);
  const currentRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (currentRef.current) {
      currentRef.current.scrollIntoView({ block: "nearest" });
    }
  }, [currentChapterPosition]);

  return (
    <div ref={listRef} className="overflow-y-auto">
      {chapters.map((chapter) => {
        const isCurrent = chapter.position === currentChapterPosition;
        const isPlayed = chapter.position < currentChapterPosition;
        return (
          <div key={chapter.position} ref={isCurrent ? currentRef : null}>
            <ChapterRow
              chapter={chapter}
              isCurrent={isCurrent}
              isPlayed={isPlayed}
              onClick={() => onSelect(chapter)}
            />
          </div>
        );
      })}
    </div>
  );
}

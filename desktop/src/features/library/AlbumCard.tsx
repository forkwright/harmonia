import clsx from "clsx";
import type { ReleaseGroup } from "../../types/api";

interface Props {
  album: ReleaseGroup;
}

export default function AlbumCard({ album }: Props) {
  const initial = album.title.charAt(0).toUpperCase();

  return (
    <div
      className={clsx(
        "bg-gray-800 rounded-lg overflow-hidden cursor-pointer",
        "hover:bg-gray-700 transition-colors group"
      )}
    >
      <div className="aspect-square bg-gray-700 flex items-center justify-center">
        <span className="text-4xl font-bold text-gray-500 group-hover:text-gray-400 select-none">
          {initial}
        </span>
      </div>
      <div className="p-3">
        <p className="text-sm font-medium text-white truncate" title={album.title}>
          {album.title}
        </p>
        <div className="flex items-center gap-2 mt-1">
          {album.year != null && (
            <span className="text-xs text-gray-400">{album.year}</span>
          )}
          <span className="text-xs text-gray-500 capitalize">{album.rg_type}</span>
        </div>
      </div>
    </div>
  );
}

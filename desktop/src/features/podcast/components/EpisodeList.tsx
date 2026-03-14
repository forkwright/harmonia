/** Virtualized list of episode rows. */

import { Virtuoso } from "react-virtuoso";
import type { Episode } from "../../../types/media";
import EpisodeRow from "./EpisodeRow";

interface Props {
  episodes: Episode[];
  onPlay: (id: string) => void;
  onEndReached: () => void;
}

export default function EpisodeList({ episodes, onPlay, onEndReached }: Props) {
  return (
    <Virtuoso
      style={{ height: "100%" }}
      data={episodes}
      endReached={onEndReached}
      itemContent={(_index, episode) => (
        <EpisodeRow key={episode.id} episode={episode} onPlay={onPlay} />
      )}
    />
  );
}

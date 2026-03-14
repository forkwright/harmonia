/** Responsive grid of podcast subscription cards. */

import { useCallback } from "react";
import { VirtuosoGrid } from "react-virtuoso";
import type { PodcastSubscription } from "../../../types/media";
import PodcastCard from "./PodcastCard";

interface Props {
  subscriptions: PodcastSubscription[];
  onEndReached: () => void;
}

export default function PodcastGrid({ subscriptions, onEndReached }: Props) {
  const endReached = useCallback(() => onEndReached(), [onEndReached]);

  return (
    <VirtuosoGrid
      style={{ height: "100%" }}
      totalCount={subscriptions.length}
      endReached={endReached}
      itemContent={(index) => (
        <div className="p-2">
          <PodcastCard podcast={subscriptions[index]} />
        </div>
      )}
      listClassName="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6"
    />
  );
}

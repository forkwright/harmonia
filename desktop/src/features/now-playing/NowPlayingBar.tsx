/** Adaptive now-playing bar: renders podcast mode when an episode is active. */

import { usePodcastStore } from "../podcast/store";
import PodcastNowPlaying from "../podcast/components/PodcastNowPlaying";
import { useQuery } from "@tanstack/react-query";
import { api } from "../../api/client";
import { useLibraryStore } from "../library/store";

export default function NowPlayingBar() {
  const token = useLibraryStore((s) => s.token);
  const currentEpisodeId = usePodcastStore((s) => s.currentEpisodeId);

  const { data: episode } = useQuery({
    queryKey: ["podcasts", "episode", currentEpisodeId, token],
    queryFn: () => api.getEpisode(currentEpisodeId!, token),
    enabled: !!currentEpisodeId && token.length > 0,
    staleTime: 60_000,
  });

  if (currentEpisodeId && episode) {
    return (
      <PodcastNowPlaying episodeTitle={episode.title} podcastTitle={episode.podcastTitle} />
    );
  }

  return (
    <span className="text-sm text-gray-500">Nothing playing</span>
  );
}

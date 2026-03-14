/** Podcast and episode media types. */

import type { ApiResponse } from "./api";

export type PaginatedResponse<T> = ApiResponse<T[]>;

export interface PodcastSubscription {
  id: string;
  title: string;
  author: string | null;
  description: string | null;
  feedUrl: string;
  imageUrl: string | null;
  episodeCount: number;
  unplayedCount: number;
  lastEpisodeDate: string | null;
  autoDownload: boolean;
  refreshIntervalMinutes: number;
}

export interface Episode {
  id: string;
  podcastId: string;
  podcastTitle: string;
  title: string;
  description: string | null;
  publishedAt: string;
  durationMs: number;
  audioUrl: string;
  fileSize: number | null;
  downloaded: boolean;
  progress: EpisodeProgress | null;
}

export interface EpisodeDetail extends Episode {
  showNotes: string | null;
  enclosureType: string;
  guid: string;
  link: string | null;
}

export interface EpisodeProgress {
  positionMs: number;
  durationMs: number;
  percentComplete: number;
  completed: boolean;
  updatedAt: string;
}

export interface EpisodeProgressUpdate {
  positionMs: number;
}

export interface EpisodeDownload {
  episodeId: string;
  episodeTitle: string;
  podcastTitle: string;
  status: "queued" | "downloading" | "completed" | "failed";
  progressPercent: number;
  fileSizeBytes: number | null;
}

export interface EpisodeQueryParams {
  page?: number;
  pageSize?: number;
  sortBy?: "published" | "title" | "duration";
  sortOrder?: "asc" | "desc";
  filter?: "all" | "unplayed" | "in_progress" | "completed" | "downloaded";
}

export interface LatestEpisodeParams {
  page?: number;
  pageSize?: number;
  hoursBack?: number;
}

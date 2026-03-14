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

export interface Audiobook {
  id: string;
  title: string;
  author: string;
  narrator: string | null;
  seriesName: string | null;
  seriesPosition: number | null;
  description: string | null;
  durationMs: number;
  coverUrl: string | null;
  chapterCount: number;
  progress: AudiobookProgress | null;
}

export interface AudiobookDetail extends Audiobook {
  publisher: string | null;
  asin: string | null;
  isbn: string | null;
  chapters: Chapter[];
}

export interface Chapter {
  position: number;
  title: string;
  startMs: number;
  endMs: number;
  source: "audnexus" | "embedded" | "fallback";
}

export interface AudiobookProgress {
  chapterPosition: number;
  offsetMs: number;
  percentComplete: number;
  chapterTitle: string;
  totalChapters: number;
  updatedAt: string;
  completedAt: string | null;
}

export interface ProgressUpdate {
  chapterPosition: number;
  offsetMs: number;
}

export interface Bookmark {
  id: string;
  audiobookId: string;
  chapterPosition: number;
  offsetMs: number;
  label: string;
  createdAt: string;
}

export interface BookmarkCreate {
  chapterPosition: number;
  offsetMs: number;
  label: string;
}

export interface AudiobookQueryParams {
  page: number;
  per_page: number;
  filter?: "all" | "in_progress" | "completed" | "not_started";
  sort?: "title" | "author" | "recently_listened" | "progress" | "date_added";
}

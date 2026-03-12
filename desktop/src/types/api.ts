export interface HealthResponse {
  status: "ok" | "degraded";
  version: string;
}

export interface Album {
  id: string;
  title: string;
  artist: string;
  year: number | null;
  trackCount: number;
  coverUrl: string | null;
}

export interface Track {
  id: string;
  title: string;
  albumId: string;
  artist: string;
  duration: number;
  trackNumber: number | null;
  url: string;
}

export interface Audiobook {
  id: string;
  title: string;
  author: string;
  duration: number;
  coverUrl: string | null;
}

export interface Podcast {
  id: string;
  title: string;
  feedUrl: string;
  episodeCount: number;
}

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  pageSize: number;
}

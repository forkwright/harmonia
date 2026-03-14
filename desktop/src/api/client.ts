import { invoke } from "@tauri-apps/api/core";
import type { ApiResponse, ListParams, ReleaseGroup, Track, Audiobook } from "../types/api";
import type {
  PodcastSubscription,
  Episode,
  EpisodeDetail,
  EpisodeProgress,
  EpisodeProgressUpdate,
  EpisodeDownload,
  EpisodeQueryParams,
  LatestEpisodeParams,
  PaginatedResponse,
} from "../types/media";

async function getBaseUrl(): Promise<string> {
  return invoke<string>("get_server_url");
}

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const baseUrl = await getBaseUrl();
  const url = `${baseUrl.replace(/\/$/, "")}${path}`;
  const response = await fetch(url, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...options?.headers,
    },
  });
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
  }
  const contentType = response.headers.get("content-type");
  if (!contentType?.includes("application/json")) {
    return undefined as unknown as T;
  }
  return response.json() as Promise<T>;
}

function buildEpisodeQuery(params: EpisodeQueryParams): URLSearchParams {
  const q = new URLSearchParams();
  if (params.page !== undefined) q.set("page", String(params.page));
  if (params.pageSize !== undefined) q.set("page_size", String(params.pageSize));
  if (params.sortBy !== undefined) q.set("sort_by", params.sortBy);
  if (params.sortOrder !== undefined) q.set("sort_order", params.sortOrder);
  if (params.filter !== undefined) q.set("filter", params.filter);
  return q;
}

function buildLatestQuery(params: LatestEpisodeParams): URLSearchParams {
  const q = new URLSearchParams();
  if (params.page !== undefined) q.set("page", String(params.page));
  if (params.pageSize !== undefined) q.set("page_size", String(params.pageSize));
  if (params.hoursBack !== undefined) q.set("hours_back", String(params.hoursBack));
  return q;
}

export const api = {
  get<T>(path: string, token?: string): Promise<T> {
    return request<T>(path, {
      method: "GET",
      headers: token ? { Authorization: `Bearer ${token}` } : undefined,
    });
  },

  post<T>(path: string, body: unknown, token?: string): Promise<T> {
    return request<T>(path, {
      method: "POST",
      body: JSON.stringify(body),
      headers: token ? { Authorization: `Bearer ${token}` } : undefined,
    });
  },

  put<T>(path: string, body: unknown, token?: string): Promise<T> {
    return request<T>(path, {
      method: "PUT",
      body: JSON.stringify(body),
      headers: token ? { Authorization: `Bearer ${token}` } : undefined,
    });
  },

  del<T>(path: string, token?: string): Promise<T> {
    return request<T>(path, {
      method: "DELETE",
      headers: token ? { Authorization: `Bearer ${token}` } : undefined,
    });
  },

  // Music library
  listReleaseGroups(params: ListParams, token: string): Promise<ApiResponse<ReleaseGroup[]>> {
    const q = new URLSearchParams({ page: String(params.page), per_page: String(params.per_page) });
    return api.get(`/api/music/release-groups?${q}`, token);
  },

  getReleaseGroup(id: string, token: string): Promise<ApiResponse<ReleaseGroup>> {
    return api.get(`/api/music/release-groups/${id}`, token);
  },

  listTracks(params: ListParams, token: string): Promise<ApiResponse<Track[]>> {
    const q = new URLSearchParams({ page: String(params.page), per_page: String(params.per_page) });
    return api.get(`/api/music/tracks?${q}`, token);
  },

  listAudiobooks(params: ListParams, token: string): Promise<ApiResponse<Audiobook[]>> {
    const q = new URLSearchParams({ page: String(params.page), per_page: String(params.per_page) });
    return api.get(`/api/audiobooks/?${q}`, token);
  },

  // Podcast subscriptions
  getSubscriptions(token: string): Promise<PodcastSubscription[]> {
    return api.get("/api/podcasts/subscriptions", token);
  },

  subscribe(feedUrl: string, token: string): Promise<PodcastSubscription> {
    return api.post("/api/podcasts/subscriptions", { feedUrl }, token);
  },

  unsubscribe(podcastId: string, token: string): Promise<void> {
    return api.del(`/api/podcasts/subscriptions/${podcastId}`, token);
  },

  updateSubscription(
    podcastId: string,
    settings: { autoDownload?: boolean; refreshIntervalMinutes?: number },
    token: string,
  ): Promise<PodcastSubscription> {
    return api.put(`/api/podcasts/subscriptions/${podcastId}`, settings, token);
  },

  refreshFeed(podcastId: string, token: string): Promise<void> {
    return api.post(`/api/podcasts/${podcastId}/refresh`, {}, token);
  },

  refreshAllFeeds(token: string): Promise<void> {
    return api.post("/api/podcasts/refresh-all", {}, token);
  },

  // Episodes
  getEpisodes(
    podcastId: string,
    params: EpisodeQueryParams,
    token: string,
  ): Promise<PaginatedResponse<Episode>> {
    const q = buildEpisodeQuery(params);
    return api.get(`/api/podcasts/${podcastId}/episodes?${q}`, token);
  },

  getEpisode(episodeId: string, token: string): Promise<EpisodeDetail> {
    return api.get(`/api/podcasts/episodes/${episodeId}`, token);
  },

  getLatestEpisodes(params: LatestEpisodeParams, token: string): Promise<PaginatedResponse<Episode>> {
    const q = buildLatestQuery(params);
    return api.get(`/api/podcasts/episodes/latest?${q}`, token);
  },

  // Playback progress
  getEpisodeProgress(episodeId: string, token: string): Promise<EpisodeProgress> {
    return api.get(`/api/podcasts/episodes/${episodeId}/progress`, token);
  },

  updateEpisodeProgress(
    episodeId: string,
    progress: EpisodeProgressUpdate,
    token: string,
  ): Promise<EpisodeProgress> {
    return api.put(`/api/podcasts/episodes/${episodeId}/progress`, progress, token);
  },

  markEpisodeCompleted(episodeId: string, token: string): Promise<void> {
    return api.post(`/api/podcasts/episodes/${episodeId}/complete`, {}, token);
  },

  markEpisodeUnplayed(episodeId: string, token: string): Promise<void> {
    return api.post(`/api/podcasts/episodes/${episodeId}/unplay`, {}, token);
  },

  // Downloads
  downloadEpisode(episodeId: string, token: string): Promise<void> {
    return api.post(`/api/podcasts/episodes/${episodeId}/download`, {}, token);
  },

  cancelDownload(episodeId: string, token: string): Promise<void> {
    return api.post(`/api/podcasts/episodes/${episodeId}/download/cancel`, {}, token);
  },

  deleteDownload(episodeId: string, token: string): Promise<void> {
    return api.del(`/api/podcasts/episodes/${episodeId}/download`, token);
  },

  getDownloadQueue(token: string): Promise<EpisodeDownload[]> {
    return api.get("/api/podcasts/downloads", token);
  },
};

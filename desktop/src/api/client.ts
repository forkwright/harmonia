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
  Audiobook as AudiobookFull,
  AudiobookDetail,
  AudiobookProgress,
  AudiobookQueryParams,
  Bookmark,
  BookmarkCreate,
  Chapter,
  ProgressUpdate,
} from "../types/media";
import type {
  ScanStatus,
  MediaItem,
  MediaItemDetail,
  MetadataUpdate,
  QualityProfile,
  QualityProfileUpdate,
  LibraryHealthReport,
  SearchQuery,
  SearchResult,
  DownloadRequest,
  DownloadStatus,
  QueueSnapshot,
  MediaRequest,
  WantedItem,
  Indexer,
  IndexerCreate,
  IndexerUpdate,
  IndexerTestResult,
  Subtitle,
  SubtitleSearchResult,
  MediaQueryParams,
  RequestQueryParams,
  RequestCreate,
  WantedQueryParams,
  WantedItemCreate,
} from "../types/management";

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

  listTracksForAlbum(albumId: string, token: string): Promise<ApiResponse<Track[]>> {
    return api.get(`/api/music/release-groups/${albumId}/tracks`, token);
  },
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

  getAudiobooks(params: AudiobookQueryParams, token: string): Promise<ApiResponse<AudiobookFull[]>> {
    const q = new URLSearchParams({
      page: String(params.page),
      per_page: String(params.per_page),
    });
    if (params.filter && params.filter !== "all") q.set("filter", params.filter);
    if (params.sort) q.set("sort", params.sort);
    return api.get(`/api/audiobooks?${q}`, token);
  },

  getAudiobook(id: string, token: string): Promise<ApiResponse<AudiobookDetail>> {
    return api.get(`/api/audiobooks/${id}`, token);
  },

  getAudiobookChapters(id: string, token: string): Promise<ApiResponse<Chapter[]>> {
    return api.get(`/api/audiobooks/${id}/chapters`, token);
  },

  getAudiobookProgress(id: string, token: string): Promise<ApiResponse<AudiobookProgress>> {
    return api.get(`/api/audiobooks/${id}/progress`, token);
  },

  updateAudiobookProgress(
    id: string,
    progress: ProgressUpdate,
    token: string,
  ): Promise<ApiResponse<AudiobookProgress>> {
    return api.put(`/api/audiobooks/${id}/progress`, progress, token);
  },

  getBookmarks(audiobookId: string, token: string): Promise<ApiResponse<Bookmark[]>> {
    return api.get(`/api/audiobooks/${audiobookId}/bookmarks`, token);
  },

  createBookmark(
    audiobookId: string,
    bookmark: BookmarkCreate,
    token: string,
  ): Promise<ApiResponse<Bookmark>> {
    return api.post(`/api/audiobooks/${audiobookId}/bookmarks`, bookmark, token);
  },

  deleteBookmark(bookmarkId: string, token: string): Promise<void> {
    return api.del(`/api/bookmarks/${bookmarkId}`, token);
  },

  // Library scanning
  triggerLibraryScan(token: string, path?: string): Promise<ScanStatus> {
    return api.post("/api/manage/scan", path ? { path } : {}, token);
  },

  getScanStatus(token: string): Promise<ScanStatus> {
    return api.get("/api/manage/scan/status", token);
  },

  // Media items
  getMediaItems(params: MediaQueryParams, token: string): Promise<PaginatedResponse<MediaItem>> {
    const q = new URLSearchParams();
    if (params.mediaType) q.set("media_type", params.mediaType);
    if (params.status) q.set("status", params.status);
    if (params.qualityTier) q.set("quality_tier", params.qualityTier);
    if (params.hasMetadata !== undefined) q.set("has_metadata", String(params.hasMetadata));
    if (params.page !== undefined) q.set("page", String(params.page));
    if (params.pageSize !== undefined) q.set("page_size", String(params.pageSize));
    if (params.sortBy) q.set("sort_by", params.sortBy);
    if (params.sortOrder) q.set("sort_order", params.sortOrder);
    return api.get(`/api/manage/media?${q}`, token);
  },

  getMediaItem(id: string, token: string): Promise<MediaItemDetail> {
    return api.get(`/api/manage/media/${id}`, token);
  },

  deleteMediaItem(id: string, token: string): Promise<void> {
    return api.del(`/api/manage/media/${id}`, token);
  },

  refreshMetadata(id: string, token: string): Promise<void> {
    return api.post(`/api/manage/media/${id}/refresh`, {}, token);
  },

  updateMetadata(id: string, metadata: MetadataUpdate, token: string): Promise<MediaItemDetail> {
    return api.put(`/api/manage/media/${id}/metadata`, metadata, token);
  },

  // Quality profiles
  getQualityProfiles(token: string): Promise<QualityProfile[]> {
    return api.get("/api/manage/quality-profiles", token);
  },

  getQualityProfile(id: string, token: string): Promise<QualityProfile> {
    return api.get(`/api/manage/quality-profiles/${id}`, token);
  },

  updateQualityProfile(
    id: string,
    profile: QualityProfileUpdate,
    token: string,
  ): Promise<QualityProfile> {
    return api.put(`/api/manage/quality-profiles/${id}`, profile, token);
  },

  // Library health
  getLibraryHealth(token: string): Promise<LibraryHealthReport> {
    return api.get("/api/manage/health", token);
  },

  // Acquisition
  searchIndexers(query: SearchQuery, token: string): Promise<SearchResult[]> {
    return api.post("/api/manage/search", query, token);
  },

  triggerDownload(req: DownloadRequest, token: string): Promise<DownloadStatus> {
    return api.post("/api/manage/downloads", req, token);
  },

  getManageDownloadQueue(token: string): Promise<QueueSnapshot> {
    return api.get("/api/manage/downloads/queue", token);
  },

  cancelManageDownload(downloadId: string, token: string): Promise<void> {
    return api.del(`/api/manage/downloads/${downloadId}`, token);
  },

  retryDownload(downloadId: string, token: string): Promise<void> {
    return api.post(`/api/manage/downloads/${downloadId}/retry`, {}, token);
  },

  // Requests
  getRequests(params: RequestQueryParams, token: string): Promise<PaginatedResponse<MediaRequest>> {
    const q = new URLSearchParams();
    if (params.status) q.set("status", params.status);
    if (params.page !== undefined) q.set("page", String(params.page));
    if (params.pageSize !== undefined) q.set("page_size", String(params.pageSize));
    return api.get(`/api/manage/requests?${q}`, token);
  },

  createRequest(request: RequestCreate, token: string): Promise<MediaRequest> {
    return api.post("/api/manage/requests", request, token);
  },

  approveRequest(requestId: string, token: string): Promise<MediaRequest> {
    return api.post(`/api/manage/requests/${requestId}/approve`, {}, token);
  },

  denyRequest(requestId: string, reason: string, token: string): Promise<MediaRequest> {
    return api.post(`/api/manage/requests/${requestId}/deny`, { reason }, token);
  },

  cancelRequest(requestId: string, token: string): Promise<void> {
    return api.del(`/api/manage/requests/${requestId}`, token);
  },

  // Wanted
  getWantedMedia(params: WantedQueryParams, token: string): Promise<PaginatedResponse<WantedItem>> {
    const q = new URLSearchParams();
    if (params.mediaType) q.set("media_type", params.mediaType);
    if (params.page !== undefined) q.set("page", String(params.page));
    if (params.pageSize !== undefined) q.set("page_size", String(params.pageSize));
    return api.get(`/api/manage/wanted?${q}`, token);
  },

  addWanted(item: WantedItemCreate, token: string): Promise<WantedItem> {
    return api.post("/api/manage/wanted", item, token);
  },

  removeWanted(id: string, token: string): Promise<void> {
    return api.del(`/api/manage/wanted/${id}`, token);
  },

  triggerWantedSearch(id: string, token: string): Promise<void> {
    return api.post(`/api/manage/wanted/${id}/search`, {}, token);
  },

  // Indexers
  getIndexers(token: string): Promise<Indexer[]> {
    return api.get("/api/manage/indexers", token);
  },

  addIndexer(config: IndexerCreate, token: string): Promise<Indexer> {
    return api.post("/api/manage/indexers", config, token);
  },

  updateIndexer(id: string, config: IndexerUpdate, token: string): Promise<Indexer> {
    return api.put(`/api/manage/indexers/${id}`, config, token);
  },

  deleteIndexer(id: string, token: string): Promise<void> {
    return api.del(`/api/manage/indexers/${id}`, token);
  },

  testIndexer(id: string, token: string): Promise<IndexerTestResult> {
    return api.post(`/api/manage/indexers/${id}/test`, {}, token);
  },

  // Subtitles
  getSubtitles(mediaId: string, token: string): Promise<Subtitle[]> {
    return api.get(`/api/manage/media/${mediaId}/subtitles`, token);
  },

  searchSubtitles(mediaId: string, token: string): Promise<SubtitleSearchResult[]> {
    return api.get(`/api/manage/media/${mediaId}/subtitles/search`, token);
  },

  downloadSubtitle(mediaId: string, subtitleId: string, token: string): Promise<void> {
    return api.post(`/api/manage/media/${mediaId}/subtitles/${subtitleId}/download`, {}, token);
  },

  deleteSubtitle(subtitleId: string, token: string): Promise<void> {
    return api.del(`/api/manage/subtitles/${subtitleId}`, token);
  },

  // Bulk actions
  bulkRefreshMetadata(ids: string[], token: string): Promise<void> {
    return api.post("/api/manage/media/bulk/refresh", { ids }, token);
  },

  bulkDelete(ids: string[], token: string): Promise<void> {
    return api.post("/api/manage/media/bulk/delete", { ids }, token);
  },

  bulkSetQualityProfile(ids: string[], qualityProfileId: string, token: string): Promise<void> {
    return api.post("/api/manage/media/bulk/quality-profile", { ids, qualityProfileId }, token);
  },
};

import type { PaginatedResponse } from "./media";

export type { PaginatedResponse };

export type MediaType =
  | "music"
  | "audiobook"
  | "ebook"
  | "podcast"
  | "manga"
  | "news"
  | "movie"
  | "tv";

export interface MediaItem {
  id: string;
  mediaType: MediaType;
  title: string;
  status: string;
  qualityScore: number | null;
  fileSize: number | null;
  filePath: string | null;
  addedAt: string;
  metadata: Record<string, unknown>;
}

export interface MediaItemDetail extends MediaItem {
  fullMetadata: Record<string, unknown>;
  qualityProfile: QualityProfile | null;
  files: MediaFile[];
  externalIds: ExternalId[];
  history: MediaEvent[];
}

export interface MediaFile {
  path: string;
  size: number;
  codec: string | null;
  quality: string | null;
}

export interface ExternalId {
  source: string;
  externalId: string;
}

export interface MediaEvent {
  type: string;
  timestamp: string;
  detail: string;
}

export interface MetadataUpdate {
  title?: string;
  [key: string]: unknown;
}

export interface ScanStatus {
  running: boolean;
  itemsScanned: number;
  itemsAdded: number;
  itemsRemoved: number;
  startedAt: string | null;
  estimatedCompletion: string | null;
}

export interface QualityProfile {
  id: string;
  name: string;
  mediaType: MediaType;
  cutoffQualityScore: number;
  upgradeAllowed: boolean;
  preferredCodecs: string[];
}

export interface QualityProfileUpdate {
  name?: string;
  cutoffQualityScore?: number;
  upgradeAllowed?: boolean;
  preferredCodecs?: string[];
}

export interface LibraryHealthReport {
  totalItems: number;
  byMediaType: Record<MediaType, number>;
  byStatus: Record<string, number>;
  qualityDistribution: QualityBucket[];
  missingMetadata: number;
  orphanedFiles: number;
  duplicates: number;
}

export interface QualityBucket {
  label: string;
  count: number;
  percentage: number;
}

export interface SearchQuery {
  query: string;
  mediaType?: MediaType;
  categoryIds?: number[];
}

export interface SearchResult {
  title: string;
  indexerName: string;
  size: number;
  seeders: number;
  leechers: number;
  quality: string | null;
  downloadUrl: string;
  protocol: "torrent" | "nzb";
  infoHash: string | null;
  publicationDate: string;
}

export interface DownloadRequest {
  searchResultUrl: string;
  mediaId?: string;
  mediaType: MediaType;
}

export interface DownloadStatus {
  id: string;
  title: string;
  status: "queued" | "downloading" | "extracting" | "importing" | "completed" | "failed";
  progressPercent: number;
  downloadSpeed: number | null;
  eta: string | null;
}

export interface QueueSnapshot {
  active: DownloadStatus[];
  queued: DownloadStatus[];
  completed: DownloadStatus[];
  failed: DownloadStatus[];
}

export interface MediaRequest {
  id: string;
  userId: string;
  userName: string;
  mediaType: MediaType;
  title: string;
  externalId: string | null;
  status: "pending" | "approved" | "denied" | "fulfilled";
  createdAt: string;
  decidedAt: string | null;
  decidedBy: string | null;
  denyReason: string | null;
}

export interface WantedItem {
  id: string;
  mediaType: MediaType;
  title: string;
  externalId: string | null;
  qualityProfileId: string;
  addedAt: string;
  lastSearchedAt: string | null;
  searchCount: number;
}

export interface Indexer {
  id: string;
  name: string;
  url: string;
  protocol: "torznab" | "newznab";
  enabled: boolean;
  status: "active" | "degraded" | "failed";
  priority: number;
  lastCheckedAt: string | null;
}

export interface IndexerCreate {
  name: string;
  url: string;
  apiKey: string;
  protocol: "torznab" | "newznab";
  priority: number;
}

export interface IndexerUpdate {
  name?: string;
  url?: string;
  apiKey?: string;
  enabled?: boolean;
  priority?: number;
}

export interface IndexerTestResult {
  success: boolean;
  responseTimeMs: number;
  categories: number;
  error: string | null;
}

export interface Subtitle {
  id: string;
  language: string;
  format: "srt" | "ass" | "ssa";
  filePath: string;
  downloadedAt: string;
}

export interface SubtitleSearchResult {
  id: string;
  language: string;
  format: string;
  rating: number;
  downloadCount: number;
}

export interface MediaQueryParams {
  mediaType?: MediaType;
  status?: string;
  qualityTier?: string;
  hasMetadata?: boolean;
  page?: number;
  pageSize?: number;
  sortBy?: "title" | "status" | "quality" | "size" | "addedAt";
  sortOrder?: "asc" | "desc";
}

export interface RequestQueryParams {
  status?: "pending" | "approved" | "denied" | "fulfilled";
  page?: number;
  pageSize?: number;
}

export interface RequestCreate {
  mediaType: MediaType;
  title: string;
  externalId?: string;
}

export interface WantedQueryParams {
  mediaType?: MediaType;
  page?: number;
  pageSize?: number;
}

export interface WantedItemCreate {
  mediaType: MediaType;
  title: string;
  externalId?: string;
  qualityProfileId: string;
}

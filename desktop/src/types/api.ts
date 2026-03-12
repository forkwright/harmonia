export interface HealthResponse {
  status: "ok" | "degraded";
  version: string;
}

export interface Meta {
  page: number;
  per_page: number;
  total: number;
}

export interface ApiResponse<T> {
  data: T;
  meta?: Meta;
  correlation_id: string;
}

export interface ReleaseGroup {
  id: string;
  title: string;
  rg_type: string;
  year: number | null;
  added_at: string;
}

export interface Track {
  id: string;
  title: string;
  position: number;
  duration_ms: number | null;
  codec: string | null;
  added_at: string;
}

export interface Audiobook {
  id: string;
  title: string;
  series_name: string | null;
  series_position: number | null;
  duration_ms: number | null;
  added_at: string;
}

export interface ListParams {
  page: number;
  per_page: number;
}

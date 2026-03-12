import { invoke } from "@tauri-apps/api/core";

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
  return response.json() as Promise<T>;
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
};

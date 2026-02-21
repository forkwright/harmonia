// Last.fm API client for similar track discovery
import type { Track } from '../types'
import { apiClient } from './client'

const LASTFM_BASE = 'https://ws.audioscrobbler.com/2.0'

function getApiKey(): string | undefined {
  return import.meta.env.VITE_LASTFM_API_KEY as string | undefined
}

export function isLastfmConfigured(): boolean {
  const key = getApiKey()
  return typeof key === 'string' && key.length > 0
}

export interface LastfmTrack {
  name: string
  artist: { name: string } | string
  match?: string
  mbid?: string
}

interface LastfmSimilarResponse {
  similartracks: {
    track: LastfmTrack[]
    '@attr': { artist: string }
  }
}

export async function fetchSimilarTracks(
  artist: string,
  title: string,
  limit = 20,
): Promise<LastfmTrack[]> {
  if (!isLastfmConfigured()) {
    throw new Error('Last.fm API key not configured')
  }


  const url = new URL(LASTFM_BASE)
  url.searchParams.set('method', 'track.getSimilar')
  url.searchParams.set('artist', artist)
  url.searchParams.set('track', title)
  url.searchParams.set('api_key', getApiKey()!)
  url.searchParams.set('format', 'json')
  url.searchParams.set('limit', String(limit))

  const response = await fetch(url.toString())
  if (!response.ok) {
    throw new Error(`Last.fm API error: ${response.status}`)
  }

  const data = (await response.json()) as LastfmSimilarResponse
  return data.similartracks?.track ?? []
}

function resolveArtistName(track: LastfmTrack): string {
  if (typeof track.artist === 'string') return track.artist
  return track.artist.name
}

// Normalize a string for fuzzy comparison: lowercase, strip punctuation, collapse whitespace
function normalize(s: string): string {
  return s
    .toLowerCase()
    .replace(/[^\w\s]/g, '')
    .replace(/\s+/g, ' ')
    .trim()
}

function isMatch(searchTitle: string, searchArtist: string, result: { title?: string; artist?: string }): boolean {
  const rt = normalize(result.title ?? '')
  const ra = normalize(result.artist ?? '')
  const st = normalize(searchTitle)
  const sa = normalize(searchArtist)
  return rt === st && ra === sa
}

// Find a library track matching a Last.fm similar track suggestion.
// Returns null if not found.
export async function findLibraryMatch(lastfmTrack: LastfmTrack): Promise<Track | null> {
  const artistName = resolveArtistName(lastfmTrack)
  const query = `${lastfmTrack.name} ${artistName}`

  try {
    const results = await apiClient.search(query, 10)
    const hit = results.find((r) =>
      isMatch(lastfmTrack.name, artistName, { title: r.title, artist: r.artist })
    )
    if (!hit) return null

    return await apiClient.getTrack(hit.trackId)
  } catch {
    return null
  }
}

// Build a list of library tracks from Last.fm similar results, skipping already-queued IDs.
export async function buildRadioTracks(
  seedTrack: Track,
  excludeIds: Set<number>,
  targetCount = 5,
): Promise<Track[]> {
  const similar = await fetchSimilarTracks(seedTrack.artist, seedTrack.title, 30)

  const matched: Track[] = []

  for (const lfm of similar) {
    if (matched.length >= targetCount) break
    const track = await findLibraryMatch(lfm)
    if (track && !excludeIds.has(track.id)) {
      matched.push(track)
      excludeIds.add(track.id)
    }
  }

  return matched
}

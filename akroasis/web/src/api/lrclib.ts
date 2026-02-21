// LRCLIB external API client
export interface LrclibResponse {
  id: number
  trackName: string
  artistName: string
  albumName: string
  duration: number
  instrumental: boolean
  plainLyrics: string | null
  syncedLyrics: string | null
}

export async function fetchLrclib(
  artist: string,
  title: string,
  album: string,
  duration: number,
): Promise<LrclibResponse | null> {
  const params = new URLSearchParams({
    artist_name: artist,
    track_name: title,
    album_name: album,
    duration: String(Math.round(duration)),
  })

  const response = await fetch(`https://lrclib.net/api/get?${params}`)

  if (response.status === 404) return null

  if (!response.ok) {
    throw new Error(`LRCLIB error ${response.status}: ${response.statusText}`)
  }

  return response.json() as Promise<LrclibResponse>
}

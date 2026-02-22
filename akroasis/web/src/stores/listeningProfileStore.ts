// Adaptive Experience — client-side listening profile
// Observes behavior and adjusts UI defaults. Never hides content, never explains itself.
import { create } from 'zustand'

// ─── Types ──────────────────────────────────────────────────────

interface AffinityWeight {
  totalPlays: number
  lastPlayedAt: string    // ISO timestamp
  last30DaysPlays: number
}

type TimeSlot = 'morning' | 'afternoon' | 'evening' | 'night'

interface ListeningProfile {
  genreWeights: Record<string, AffinityWeight>
  artistWeights: Record<string, AffinityWeight>
  timePatterns: Record<TimeSlot, Record<string, number>>  // slot → genre → count
  featureUsage: Record<string, { uses: number; lastUsed: string }>
  totalListeningMs: number
  firstSeen: string
  lastUpdated: string
}

interface ListeningProfileState extends ListeningProfile {
  // Record a track play (call after >30s of playback)
  recordPlay: (track: { artist: string; genre?: string; duration: number }) => void
  // Record feature/page usage
  recordFeatureUse: (feature: string) => void
  // Get top genre for current time slot (or overall)
  getTopGenre: (forTimeSlot?: boolean) => string | null
  // Get top N artists by decayed weight
  getTopArtists: (n: number) => string[]
  // Get current time slot
  getCurrentTimeSlot: () => TimeSlot
  // Compute decayed weight for a genre/artist
  getDecayedWeight: (weight: AffinityWeight) => number
  // Check if enough data for a specific adaptation
  hasConfidence: (type: 'genreDefault' | 'timeOfDay' | 'navDeEmphasis' | 'searchWeight', key?: string) => boolean
  // Get nav item emphasis (0 = muted, 1 = normal)
  getNavEmphasis: (feature: string) => number
  // Get suggested genres for current time slot
  getSuggestedGenres: (limit?: number) => string[]
  // Run decay maintenance
  runDecay: () => void
}

const STORAGE_KEY = 'akroasis_profile'
const MAX_GENRES = 200
const MAX_ARTISTS = 500

// ─── Time Helpers ───────────────────────────────────────────────

function getTimeSlot(hour?: number): TimeSlot {
  const h = hour ?? new Date().getHours()
  if (h >= 5 && h < 12) return 'morning'
  if (h >= 12 && h < 18) return 'afternoon'
  if (h >= 18 && h < 23) return 'evening'
  return 'night'
}

function daysSince(isoDate: string): number {
  return (Date.now() - new Date(isoDate).getTime()) / (1000 * 60 * 60 * 24)
}

function isWithinDays(isoDate: string, days: number): boolean {
  return daysSince(isoDate) <= days
}

// ─── Decay ──────────────────────────────────────────────────────

function decayWeight(w: AffinityWeight): number {
  const days = daysSince(w.lastPlayedAt)
  if (days < 7) return w.last30DaysPlays * 1.0
  if (days < 30) return w.last30DaysPlays * 0.7
  if (days < 90) return w.totalPlays * 0.3
  if (days < 180) return w.totalPlays * 0.1
  return 0
}

// ─── Persistence ────────────────────────────────────────────────

function loadProfile(): ListeningProfile {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored) {
      const parsed = JSON.parse(stored)
      return {
        genreWeights: parsed.genreWeights ?? {},
        artistWeights: parsed.artistWeights ?? {},
        timePatterns: parsed.timePatterns ?? { morning: {}, afternoon: {}, evening: {}, night: {} },
        featureUsage: parsed.featureUsage ?? {},
        totalListeningMs: parsed.totalListeningMs ?? 0,
        firstSeen: parsed.firstSeen ?? new Date().toISOString(),
        lastUpdated: parsed.lastUpdated ?? new Date().toISOString(),
      }
    }
  } catch { /* ignore corrupt data */ }
  return {
    genreWeights: {},
    artistWeights: {},
    timePatterns: { morning: {}, afternoon: {}, evening: {}, night: {} },
    featureUsage: {},
    totalListeningMs: 0,
    firstSeen: new Date().toISOString(),
    lastUpdated: new Date().toISOString(),
  }
}

function saveProfile(profile: ListeningProfile) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(profile))
  } catch { /* localStorage full — degrade gracefully */ }
}

// ─── Pruning ────────────────────────────────────────────────────

function pruneMap<T extends AffinityWeight>(
  map: Record<string, T>,
  maxEntries: number,
): Record<string, T> {
  const entries = Object.entries(map)
  if (entries.length <= maxEntries) return map
  // Sort by decayed weight, keep top N
  entries.sort(([, a], [, b]) => decayWeight(b) - decayWeight(a))
  return Object.fromEntries(entries.slice(0, maxEntries))
}

// ─── Store ──────────────────────────────────────────────────────

export const useListeningProfileStore = create<ListeningProfileState>((set, get) => {
  const initial = loadProfile()

  return {
    ...initial,

    recordPlay: (track) => {
      const now = new Date().toISOString()
      const slot = getTimeSlot()
      const genre = track.genre || 'Unknown'

      set((state) => {
        // Update genre weights
        const gw = { ...state.genreWeights }
        const existing = gw[genre] ?? { totalPlays: 0, lastPlayedAt: now, last30DaysPlays: 0 }
        gw[genre] = {
          totalPlays: existing.totalPlays + 1,
          lastPlayedAt: now,
          last30DaysPlays: isWithinDays(existing.lastPlayedAt, 30)
            ? existing.last30DaysPlays + 1
            : 1,
        }

        // Update artist weights
        const aw = { ...state.artistWeights }
        const existingArtist = aw[track.artist] ?? { totalPlays: 0, lastPlayedAt: now, last30DaysPlays: 0 }
        aw[track.artist] = {
          totalPlays: existingArtist.totalPlays + 1,
          lastPlayedAt: now,
          last30DaysPlays: isWithinDays(existingArtist.lastPlayedAt, 30)
            ? existingArtist.last30DaysPlays + 1
            : 1,
        }

        // Update time patterns
        const tp = { ...state.timePatterns }
        tp[slot] = { ...tp[slot] }
        tp[slot][genre] = (tp[slot][genre] ?? 0) + 1

        const newState = {
          genreWeights: pruneMap(gw, MAX_GENRES),
          artistWeights: pruneMap(aw, MAX_ARTISTS),
          timePatterns: tp,
          totalListeningMs: state.totalListeningMs + (track.duration * 1000),
          lastUpdated: now,
        }

        // Save async
        saveProfile({ ...state, ...newState })

        return newState
      })
    },

    recordFeatureUse: (feature) => {
      const now = new Date().toISOString()
      set((state) => {
        const fu = { ...state.featureUsage }
        const existing = fu[feature] ?? { uses: 0, lastUsed: now }
        fu[feature] = { uses: existing.uses + 1, lastUsed: now }

        const newState = { featureUsage: fu, lastUpdated: now }
        saveProfile({ ...state, ...newState })
        return newState
      })
    },

    getCurrentTimeSlot: () => getTimeSlot(),

    getDecayedWeight: (weight) => decayWeight(weight),

    getTopGenre: (forTimeSlot = false) => {
      const state = get()
      if (forTimeSlot) {
        const slot = getTimeSlot()
        const patterns = state.timePatterns[slot] ?? {}
        const entries = Object.entries(patterns).filter(([, count]) => count >= 10)
        if (entries.length === 0) return null
        entries.sort(([, a], [, b]) => b - a)
        return entries[0][0]
      }
      // Overall top genre by decayed weight
      const entries = Object.entries(state.genreWeights)
      if (entries.length === 0) return null
      entries.sort(([, a], [, b]) => decayWeight(b) - decayWeight(a))
      const top = entries[0]
      if (decayWeight(top[1]) === 0) return null
      return top[0]
    },

    getTopArtists: (n) => {
      const entries = Object.entries(get().artistWeights)
      entries.sort(([, a], [, b]) => decayWeight(b) - decayWeight(a))
      return entries.slice(0, n).filter(([, w]) => decayWeight(w) > 0).map(([name]) => name)
    },

    hasConfidence: (type, key) => {
      const state = get()
      const appAgeDays = daysSince(state.firstSeen)

      switch (type) {
        case 'genreDefault': {
          if (!key) return false
          const gw = state.genreWeights[key]
          return !!gw && gw.last30DaysPlays >= 10
        }
        case 'timeOfDay': {
          const slot = getTimeSlot()
          const patterns = state.timePatterns[slot] ?? {}
          const total = Object.values(patterns).reduce((s, c) => s + c, 0)
          return total >= 10
        }
        case 'navDeEmphasis':
          return appAgeDays >= 30
        case 'searchWeight': {
          if (!key) return false
          const aw = state.artistWeights[key]
          return !!aw && aw.totalPlays >= 5 && daysSince(aw.lastPlayedAt) <= 90
        }
        default:
          return false
      }
    },

    getNavEmphasis: (feature) => {
      const state = get()
      if (!get().hasConfidence('navDeEmphasis')) return 1 // Too early to de-emphasize
      const fu = state.featureUsage[feature]
      if (!fu) return 0.6 // Never used → subtle de-emphasis
      if (daysSince(fu.lastUsed) > 30) return 0.7 // Not used in 30 days
      return 1 // Used recently → full emphasis
    },

    getSuggestedGenres: (limit = 6) => {
      const state = get()
      const slot = getTimeSlot()
      const patterns = state.timePatterns[slot] ?? {}
      const entries = Object.entries(patterns)
      if (entries.length === 0) return []

      // Only suggest genres with confidence
      const confident = entries.filter(([, count]) => count >= 10)
      confident.sort(([, a], [, b]) => b - a)
      return confident.slice(0, limit).map(([genre]) => genre)
    },

    runDecay: () => {
      set((state) => {
        // Prune entries with zero decayed weight
        const gw: Record<string, AffinityWeight> = {}
        for (const [key, val] of Object.entries(state.genreWeights)) {
          if (decayWeight(val) > 0) gw[key] = val
        }
        const aw: Record<string, AffinityWeight> = {}
        for (const [key, val] of Object.entries(state.artistWeights)) {
          if (decayWeight(val) > 0) aw[key] = val
        }

        // Reset last30DaysPlays for stale entries
        for (const w of Object.values(gw)) {
          if (!isWithinDays(w.lastPlayedAt, 30)) w.last30DaysPlays = 0
        }
        for (const w of Object.values(aw)) {
          if (!isWithinDays(w.lastPlayedAt, 30)) w.last30DaysPlays = 0
        }

        const newState = { genreWeights: gw, artistWeights: aw, lastUpdated: new Date().toISOString() }
        saveProfile({ ...state, ...newState })
        return newState
      })
    },
  }
})

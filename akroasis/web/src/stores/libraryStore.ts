// Library browsing state: view, filters, sort, facets
import { create } from 'zustand'
import { apiClient } from '../api/client'
import { logError } from '../utils/errorLogger'
import type {
  Artist, Album, Track,
  LibraryFacets, FilterCondition,
} from '../types'

export type LibraryView = 'artists' | 'albums' | 'tracks' | 'genres'
export type SortField = 'title' | 'artist' | 'year' | 'name' | 'albumCount' | 'trackCount' | 'duration' | 'format' | 'sampleRate'
export type SortDirection = 'asc' | 'desc'

interface LibraryState {
  // View
  view: LibraryView
  setView: (view: LibraryView) => void

  // Data
  artists: Artist[]
  albums: Album[]
  tracks: Track[]
  isLoading: boolean
  error: string | null
  totalCount: number
  page: number
  hasMore: boolean

  // Facets
  facets: LibraryFacets | null
  facetsLoading: boolean

  // Filters
  activeFilters: FilterCondition[]
  addFilter: (condition: FilterCondition) => void
  removeFilter: (field: string) => void
  clearFilters: () => void

  // Sort
  sortField: SortField
  sortDirection: SortDirection
  setSort: (field: SortField, direction?: SortDirection) => void

  // Drill-down
  selectedArtist: Artist | null
  selectedAlbum: Album | null
  selectedGenre: string | null

  // Actions
  fetchFacets: () => Promise<void>
  fetchArtists: (page?: number) => Promise<void>
  fetchAlbums: (artistId?: number, page?: number) => Promise<void>
  fetchTracks: (albumId?: number, page?: number) => Promise<void>
  fetchFiltered: (page?: number) => Promise<void>
  selectArtist: (artist: Artist) => Promise<void>
  selectAlbum: (album: Album) => Promise<void>
  selectGenre: (genre: string) => void
  goBack: () => void
  loadMore: () => Promise<void>
}

const PAGE_SIZE = 50
const STORAGE_KEY = 'akroasis_library'

// --- Client-side sort helpers ---
function sortArray<T>(items: T[], field: string, direction: SortDirection): T[] {
  const sorted = [...items]
  const dir = direction === 'asc' ? 1 : -1

  sorted.sort((a, b) => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const av = (a as any)[field]
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const bv = (b as any)[field]

    // Nulls to end
    if (av == null && bv == null) return 0
    if (av == null) return 1
    if (bv == null) return -1

    if (typeof av === 'string' && typeof bv === 'string') {
      return dir * av.localeCompare(bv, undefined, { sensitivity: 'base' })
    }
    if (typeof av === 'number' && typeof bv === 'number') {
      return dir * (av - bv)
    }
    return 0
  })
  return sorted
}

function sortArtists(artists: Artist[], field: SortField, direction: SortDirection): Artist[] {
  const fieldMap: Record<string, string> = { name: 'name', albumCount: 'albumCount', trackCount: 'trackCount' }
  return sortArray(artists, fieldMap[field] || 'name', direction)
}

function sortAlbums(albums: Album[], field: SortField, direction: SortDirection): Album[] {
  const fieldMap: Record<string, string> = { title: 'title', artist: 'artist', year: 'year', duration: 'duration', trackCount: 'trackCount' }
  return sortArray(albums, fieldMap[field] || 'title', direction)
}

function sortTracks(tracks: Track[], field: SortField, direction: SortDirection): Track[] {
  const fieldMap: Record<string, string> = {
    title: 'title', artist: 'artist', duration: 'duration',
    format: 'format', sampleRate: 'sampleRate', year: 'album'
  }
  return sortArray(tracks, fieldMap[field] || 'title', direction)
}

function loadPrefs(): { view: LibraryView; sortField: SortField; sortDirection: SortDirection } {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored) return JSON.parse(stored)
  } catch { /* ignore */ }
  return { view: 'artists', sortField: 'name', sortDirection: 'asc' }
}

function savePrefs(view: LibraryView, sortField: SortField, sortDirection: SortDirection) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify({ view, sortField, sortDirection }))
}

export const useLibraryStore = create<LibraryState>((set, get) => {
  const prefs = loadPrefs()

  return {
    view: prefs.view,
    artists: [],
    albums: [],
    tracks: [],
    isLoading: false,
    error: null,
    totalCount: 0,
    page: 1,
    hasMore: false,
    facets: null,
    facetsLoading: false,
    activeFilters: [],
    sortField: prefs.sortField,
    sortDirection: prefs.sortDirection,
    selectedArtist: null,
    selectedAlbum: null,
    selectedGenre: null,

    setView: (view) => {
      set({ view, page: 1, selectedArtist: null, selectedAlbum: null, selectedGenre: null })
      savePrefs(view, get().sortField, get().sortDirection)
    },

    addFilter: (condition) => {
      const existing = get().activeFilters.filter(f => f.field !== condition.field)
      set({ activeFilters: [...existing, condition], page: 1 })
      get().fetchFiltered()
    },

    removeFilter: (field) => {
      set({ activeFilters: get().activeFilters.filter(f => f.field !== field), page: 1 })
      const filters = get().activeFilters
      if (filters.length > 0) {
        get().fetchFiltered()
      } else {
        // No filters left — reload the current view normally
        const view = get().view
        if (view === 'artists') get().fetchArtists()
        else if (view === 'albums') get().fetchAlbums()
        else if (view === 'tracks') get().fetchTracks()
      }
    },

    clearFilters: () => {
      set({ activeFilters: [], page: 1, selectedGenre: null })
      const view = get().view
      if (view === 'artists') get().fetchArtists()
      else if (view === 'albums') get().fetchAlbums()
      else if (view === 'tracks') get().fetchTracks()
    },

    setSort: (field, direction) => {
      const dir = direction ?? (get().sortField === field && get().sortDirection === 'asc' ? 'desc' : 'asc')
      set({ sortField: field, sortDirection: dir })
      savePrefs(get().view, field, dir)

      // Re-sort current data in-memory
      const { view, artists, albums, tracks } = get()
      if (view === 'artists' && artists.length > 0) {
        set({ artists: sortArtists(artists, field, dir) })
      } else if (view === 'albums' && albums.length > 0) {
        set({ albums: sortAlbums(albums, field, dir) })
      } else if (view === 'tracks' && tracks.length > 0) {
        set({ tracks: sortTracks(tracks, field, dir) })
      }
    },

    fetchFacets: async () => {
      if (get().facets) return // Cache — facets rarely change
      set({ facetsLoading: true })
      try {
        const facets = await apiClient.getFacets()
        set({ facets, facetsLoading: false })
      } catch {
        set({ facetsLoading: false })
      }
    },

    fetchArtists: async (page = 1) => {
      set({ isLoading: true, error: null })
      try {
        const data = await apiClient.getArtists(page, PAGE_SIZE)
        const { sortField, sortDirection } = get()
        const merged = page === 1 ? data.items : [...get().artists, ...data.items]
        set({
          artists: sortArtists(merged, sortField, sortDirection),
          totalCount: data.totalCount,
          page,
          hasMore: page * PAGE_SIZE < data.totalCount,
          isLoading: false,
        })
      } catch (err) {
        logError('library', 'Failed to load artists', err); set({ error: err instanceof Error ? err.message : 'Failed to load artists', isLoading: false })
      }
    },

    fetchAlbums: async (artistId, _page = 1) => {
      set({ isLoading: true, error: null })
      try {
        const { sortField, sortDirection } = get()
        if (artistId) {
          const data = await apiClient.getAlbums(artistId)
          set({ albums: sortAlbums(data, sortField, sortDirection), totalCount: data.length, page: 1, hasMore: false, isLoading: false })
        } else {
          // All albums — use the real albums endpoint with proper IDs
          const result = await apiClient.getAlbums()
          set({
            albums: sortAlbums(result.items, sortField, sortDirection),
            totalCount: result.totalCount,
            page: 1,
            hasMore: false,
            isLoading: false,
          })
        }
      } catch (err) {
        logError('library', 'Failed to load albums', err); set({ error: err instanceof Error ? err.message : 'Failed to load albums', isLoading: false })
      }
    },

    fetchTracks: async (albumId, page = 1) => {
      set({ isLoading: true, error: null })
      try {
        const { sortField, sortDirection } = get()
        if (albumId) {
          const data = await apiClient.getTracks(albumId)
          set({ tracks: sortTracks(data, sortField, sortDirection), totalCount: data.length, page: 1, hasMore: false, isLoading: false })
        } else {
          const result = await apiClient.filterLibrary({
            conditions: get().activeFilters,
            logic: 'and',
            page,
            pageSize: PAGE_SIZE,
          })
          set({
            tracks: page === 1 ? result.items : [...get().tracks, ...result.items],
            totalCount: result.totalCount,
            page,
            hasMore: page * PAGE_SIZE < result.totalCount,
            isLoading: false,
          })
        }
      } catch (err) {
        logError('library', 'Failed to load tracks', err); set({ error: err instanceof Error ? err.message : 'Failed to load tracks', isLoading: false })
      }
    },

    fetchFiltered: async (page = 1) => {
      set({ isLoading: true, error: null })
      try {
        const result = await apiClient.filterLibrary({
          conditions: get().activeFilters,
          logic: 'and',
          page,
          pageSize: PAGE_SIZE,
        })
        set({
          tracks: page === 1 ? result.items : [...get().tracks, ...result.items],
          totalCount: result.totalCount,
          page,
          hasMore: page * PAGE_SIZE < result.totalCount,
          isLoading: false,
        })
      } catch (err) {
        logError('library', 'Failed to filter library', err); set({ error: err instanceof Error ? err.message : 'Failed to filter library', isLoading: false })
      }
    },

    selectArtist: async (artist) => {
      set({ selectedArtist: artist, view: 'albums' })
      await get().fetchAlbums(artist.id)
    },

    selectAlbum: async (album) => {
      set({ selectedAlbum: album, view: 'tracks' })
      await get().fetchTracks(album.id)

      // Compute album duration/trackCount from fetched tracks if API returned 0
      const tracks = get().tracks
      if (tracks.length > 0 && (!album.duration || !album.trackCount)) {
        const computedDuration = tracks.reduce((sum, t) => sum + (t.duration || 0), 0)
        set({
          selectedAlbum: {
            ...album,
            duration: album.duration || computedDuration,
            trackCount: album.trackCount || tracks.length,
          }
        })
      }
    },

    selectGenre: (genre) => {
      set({ selectedGenre: genre })
      get().addFilter({ field: 'Genre', operator: 'contains', value: genre })
    },

    goBack: () => {
      const { view, selectedArtist } = get()
      if (view === 'tracks' && get().selectedAlbum) {
        set({ selectedAlbum: null, view: 'albums' })
        if (selectedArtist) get().fetchAlbums(selectedArtist.id)
      } else if (view === 'albums' && selectedArtist) {
        set({ selectedArtist: null, view: 'artists' })
        get().fetchArtists()
      } else if (get().selectedGenre) {
        set({ selectedGenre: null })
        get().removeFilter('genres')
      }
    },

    loadMore: async () => {
      const { page, hasMore, isLoading, view, activeFilters } = get()
      if (!hasMore || isLoading) return
      const nextPage = page + 1
      if (activeFilters.length > 0) {
        await get().fetchFiltered(nextPage)
      } else if (view === 'artists') {
        await get().fetchArtists(nextPage)
      } else if (view === 'tracks') {
        await get().fetchTracks(undefined, nextPage)
      }
    },
  }
})

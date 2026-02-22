// Library browsing state: view, filters, sort, facets
import { create } from 'zustand'
import { apiClient } from '../api/client'
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
        set({
          artists: page === 1 ? data.items : [...get().artists, ...data.items],
          totalCount: data.totalCount,
          page,
          hasMore: page * PAGE_SIZE < data.totalCount,
          isLoading: false,
        })
      } catch (err) {
        set({ error: err instanceof Error ? err.message : 'Failed to load artists', isLoading: false })
      }
    },

    fetchAlbums: async (artistId, page = 1) => {
      set({ isLoading: true, error: null })
      try {
        if (artistId) {
          const data = await apiClient.getAlbums(artistId)
          set({ albums: data, totalCount: data.length, page: 1, hasMore: false, isLoading: false })
        } else {
          // All albums — use filter endpoint with no conditions for pagination
          const result = await apiClient.filterLibrary({
            conditions: get().activeFilters,
            logic: 'and',
            page,
            pageSize: PAGE_SIZE,
          })
          // Map tracks to albums (group by album)
          const albumMap = new Map<string, Album>()
          for (const track of result.items) {
            const key = `${track.artist}|${track.album}`
            if (!albumMap.has(key)) {
              albumMap.set(key, {
                id: track.id, // Use first track ID as album ID
                title: track.album,
                artist: track.artist,
                trackCount: 1,
                duration: track.duration,
                coverArtUrl: track.coverArtUrl,
              })
            } else {
              const album = albumMap.get(key)!
              album.trackCount++
              album.duration += track.duration
            }
          }
          const albums = Array.from(albumMap.values())
          set({
            albums: page === 1 ? albums : [...get().albums, ...albums],
            totalCount: result.totalCount,
            page,
            hasMore: page * PAGE_SIZE < result.totalCount,
            isLoading: false,
          })
        }
      } catch (err) {
        set({ error: err instanceof Error ? err.message : 'Failed to load albums', isLoading: false })
      }
    },

    fetchTracks: async (albumId, page = 1) => {
      set({ isLoading: true, error: null })
      try {
        if (albumId) {
          const data = await apiClient.getTracks(albumId)
          set({ tracks: data, totalCount: data.length, page: 1, hasMore: false, isLoading: false })
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
        set({ error: err instanceof Error ? err.message : 'Failed to load tracks', isLoading: false })
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
        set({ error: err instanceof Error ? err.message : 'Failed to filter library', isLoading: false })
      }
    },

    selectArtist: async (artist) => {
      set({ selectedArtist: artist, view: 'albums' })
      await get().fetchAlbums(artist.id)
    },

    selectAlbum: async (album) => {
      set({ selectedAlbum: album, view: 'tracks' })
      await get().fetchTracks(album.id)
    },

    selectGenre: (genre) => {
      set({ selectedGenre: genre })
      get().addFilter({ field: 'genres', operator: 'contains', value: genre })
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

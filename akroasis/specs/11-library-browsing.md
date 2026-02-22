# Spec 11: Multi-Axis Library Browsing

**Status:** Draft
**Priority:** High
**Depends On:** Spec 08 (design system for consistent components)

## Goal

Transform the Library from a single drill-down path (artist → album → tracks) into a multi-axis browser that lets users enter from any angle: artists, albums, tracks, or genres. A shared filter bar provides faceted refinement across all views. The backend already supports this — `POST /api/v3/library/filter` accepts conditional queries with AND/OR logic, and `GET /api/v3/library/facets` returns available genres, formats, sample rates, bit depths, dynamic range, and year ranges. The web UI uses none of it.

## Design Philosophy

The Library should feel like a record collection, not a database query tool. Browsing by genre should feel like walking to a different shelf. Filtering by format should feel like pulling out the vinyl section. The controls exist but don't demand attention — you notice them when you need them.

## Phases

### Phase 1: View Tabs

Replace the single artist view with tabbed navigation at the top of the Library page.

```
  Artists    Albums    Tracks    Genres
  ────────────────────────────────────
```

- [ ] Tab bar component at top of Library page (persistent, not in nav)
- [ ] **Artists view** — current grid, unchanged
- [ ] **Albums view** — all albums in a grid, sortable by title / artist / year / recently added
- [ ] **Tracks view** — table layout: #, title, artist, album, format, duration. Sortable columns. Click to play.
- [ ] **Genres view** — genre cards (fetched from facets API). Click genre → filtered album grid for that genre
- [ ] URL reflects current view: `/library/artists`, `/library/albums`, `/library/tracks`, `/library/genres`
- [ ] Remember last-used view in localStorage

### Phase 2: Facet Filter Bar

A collapsible filter bar shared across all views. Populated from `GET /api/v3/library/facets`.

```
  Filters ▾   Genre: [All ▾]   Format: [All ▾]   Year: [1965 — 2025]   Quality: [All ▾]
```

- [ ] Add `getFacets()` method to apiClient, returning facets resource
- [ ] Add `useLibraryStore` (Zustand) holding current view, active filters, sort order, pagination
- [ ] Filter bar component: dropdowns for genre, format, bit depth. Range slider for year, dynamic range
- [ ] Filters translate to `POST /api/v3/library/filter` conditions:
  - Genre "Rock" → `{ field: "genres", operator: "contains", value: "Rock" }`
  - Format "FLAC" → `{ field: "audioFormat", operator: "equals", value: "flac" }`
  - Year 2000-2024 → two conditions with `greaterThanOrEqual` and `lessThanOrEqual`
  - Bit depth 24 → `{ field: "bitDepth", operator: "equals", value: "24" }`
- [ ] Active filter pills shown below the bar with × to remove
- [ ] Clear all button
- [ ] Filter bar collapses to single line when no filters active, expands on click

### Phase 3: Sort & Display Options

- [ ] Sort dropdown per view: Artists (name, album count), Albums (title, artist, year, added), Tracks (title, artist, album, duration, format)
- [ ] Sort direction toggle (asc/desc)
- [ ] Grid/list toggle for Albums view (grid = cover art focus, list = information density)
- [ ] Infinite scroll or "load more" pagination (backend supports `page` and `pageSize`)
- [ ] Persist sort preference per view in localStorage

### Phase 4: Genre Browsing

Genre deserves its own design treatment because genres are discovery, not just filtering.

- [ ] Genre cards: styled tiles with genre name and album count. No generic icons — text is enough
- [ ] Click genre → shows albums in that genre, with breadcrumb: `Genres > Rock`
- [ ] Sub-genre awareness: if the library has both "Rock" and "Alternative Rock," show them separately (don't try to be clever with merging)
- [ ] Genre view respects other active filters (format, year, quality) — genre IS a filter, composed with others
- [ ] Consider: genre accent colors derived from genre name hash (deterministic, subtle background tint). Only if it doesn't feel arbitrary

### Phase 5: Search Integration

- [ ] Cmd+K search results grouped by type: Artists, Albums, Tracks
- [ ] Search uses backend filter API with `contains` operator on title/name fields
- [ ] Recent searches persisted in localStorage (last 5)
- [ ] Search from within a genre/filter context: pre-scoped but clearable

## API Integration

### Facets Endpoint (existing)
```
GET /api/v3/library/facets
→ {
    formats: ["flac", "mp3", "aac"],
    sampleRates: [44100, 48000, 96000],
    bitDepths: [16, 24, 32],
    genres: ["Alternative", "Country", "Rock", ...],
    dynamicRangeRange: { min: 2, max: 18 },
    yearRange: { min: 1965, max: 2025 }
  }
```

### Filter Endpoint (existing)
```
POST /api/v3/library/filter
Body: {
  conditions: [
    { field: "genres", operator: "contains", value: "Country" },
    { field: "audioFormat", operator: "equals", value: "flac" },
    { field: "bitDepth", operator: "greaterThanOrEqual", value: "24" }
  ],
  logic: "and",
  page: 1,
  pageSize: 50
}
→ {
    items: [...],
    page: 1,
    pageSize: 50,
    totalCount: 47,
    summary: { avgDynamicRange: 12.3, formatDistribution: {...}, ... }
  }
```

## Dependencies

- `GET /api/v3/library/facets` — **exists** in Mouseion (FacetsController.cs)
- `POST /api/v3/library/filter` — **exists** in Mouseion (LibraryController.cs + FilterQueryBuilder.cs)
- Neither endpoint is currently called from the web frontend

## Notes

- The current Library loads all artists at once (`getArtists(1, 50)`). This works for small libraries but won't scale. The filter API with pagination is the right long-term approach.
- Albums in the backend have a `Genres` field (JSON array). The facets controller already parses these into unique genre lists with deduplication.
- The filter summary (avg dynamic range, format distribution) returned by the filter endpoint could power subtle UI indicators — e.g., a small "47 albums · 83% lossless" counter in the filter bar.
- Don't over-design the filter bar. Roon's filtering is powerful but intimidating. Spotify's is invisible but limiting. The sweet spot: visible when you want it, ignorable when you don't.
- Track table view should use `tabular-nums` for all numeric columns (duration, bitrate, sample rate). Alignment matters for scannability.

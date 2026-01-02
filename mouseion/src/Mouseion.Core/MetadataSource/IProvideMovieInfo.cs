// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Movies;

namespace Mouseion.Core.MetadataSource;

public interface IProvideMovieInfo
{
    Task<Movie?> GetByTmdbIdAsync(int tmdbId, CancellationToken ct = default);
    Task<Movie?> GetByImdbIdAsync(string imdbId, CancellationToken ct = default);
    Task<List<Movie>> SearchByTitleAsync(string title, int? year = null, CancellationToken ct = default);
    Task<List<Movie>> GetTrendingAsync(CancellationToken ct = default);
    Task<List<Movie>> GetPopularAsync(CancellationToken ct = default);
    Task<List<Movie>> GetUpcomingAsync(CancellationToken ct = default);
    Task<List<Movie>> GetNowPlayingAsync(CancellationToken ct = default);

    Movie? GetByTmdbId(int tmdbId);
    Movie? GetByImdbId(string imdbId);
    List<Movie> SearchByTitle(string title, int? year = null);
    List<Movie> GetTrending();
    List<Movie> GetPopular();
    List<Movie> GetUpcoming();
    List<Movie> GetNowPlaying();
}

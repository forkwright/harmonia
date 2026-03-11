// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.Movies;
using Mouseion.Core.TV;
using Mouseion.Core.Music;

namespace Mouseion.Core.Webhooks;

/// <summary>
/// Resolves external media server item IDs to Mouseion MediaItem IDs
/// by matching on shared metadata identifiers (TMDB, TVDB, IMDB, MusicBrainz).
/// </summary>
public interface IExternalMediaResolver
{
    Task<int?> ResolveAsync(ExternalMediaInfo info, CancellationToken ct = default);
}

public class ExternalMediaResolver : IExternalMediaResolver
{
    private readonly MovieRepository _movieRepo;
    private readonly SeriesRepository _seriesRepo;
    private readonly ArtistRepository _artistRepo;
    private readonly ILogger<ExternalMediaResolver> _logger;

    public ExternalMediaResolver(
        MovieRepository movieRepo,
        SeriesRepository seriesRepo,
        ArtistRepository artistRepo,
        ILogger<ExternalMediaResolver> logger)
    {
        _movieRepo = movieRepo;
        _seriesRepo = seriesRepo;
        _artistRepo = artistRepo;
        _logger = logger;
    }

    public async Task<int?> ResolveAsync(ExternalMediaInfo info, CancellationToken ct = default)
    {
        // Try TMDB ID first (movies)
        if (!string.IsNullOrWhiteSpace(info.TmdbId))
        {
            var movie = await _movieRepo.FindByTmdbIdAsync(info.TmdbId, ct).ConfigureAwait(false);
            if (movie != null)
            {
                _logger.LogDebug("Resolved TMDB {TmdbId} to MediaItem {Id}", info.TmdbId, movie.Id);
                return movie.Id;
            }
        }

        // Try IMDB ID (movies)
        if (!string.IsNullOrWhiteSpace(info.ImdbId))
        {
            var movie = await _movieRepo.FindByImdbIdAsync(info.ImdbId, ct).ConfigureAwait(false);
            if (movie != null)
            {
                _logger.LogDebug("Resolved IMDB {ImdbId} to MediaItem {Id}", info.ImdbId, movie.Id);
                return movie.Id;
            }
        }

        // Try TVDB ID (series — returns series, not episode)
        if (info.TvdbId.HasValue)
        {
            var series = await _seriesRepo.FindByTvdbIdAsync(info.TvdbId.Value, ct).ConfigureAwait(false);
            if (series != null)
            {
                _logger.LogDebug("Resolved TVDB {TvdbId} to Series {Id}", info.TvdbId.Value, series.Id);
                // TODO: resolve to specific episode via season/episode numbers if provided
                return series.Id;
            }
        }

        // Try MusicBrainz ID (artists/albums)
        if (!string.IsNullOrWhiteSpace(info.MusicBrainzId))
        {
            var artist = await _artistRepo.FindByMusicBrainzIdAsync(info.MusicBrainzId, ct).ConfigureAwait(false);
            if (artist != null)
            {
                _logger.LogDebug("Resolved MusicBrainz {MbId} to Artist {Id}", info.MusicBrainzId, artist.Id);
                return artist.Id;
            }
        }

        _logger.LogDebug("Could not resolve external media: TMDB={TmdbId}, IMDB={ImdbId}, TVDB={TvdbId}, MB={MbId}",
            info.TmdbId, info.ImdbId, info.TvdbId, info.MusicBrainzId);
        return null;
    }
}

public class ExternalMediaInfo
{
    public string? TmdbId { get; set; }
    public string? ImdbId { get; set; }
    public int? TvdbId { get; set; }
    public string? MusicBrainzId { get; set; }
    public string? Title { get; set; }
    public int? Year { get; set; }
    public int? SeasonNumber { get; set; }
    public int? EpisodeNumber { get; set; }
}

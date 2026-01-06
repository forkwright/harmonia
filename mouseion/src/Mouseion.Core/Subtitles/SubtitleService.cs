// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Common.Extensions;
using Mouseion.Core.MediaFiles;
using Mouseion.Core.Movies;

namespace Mouseion.Core.Subtitles;

public interface ISubtitleService
{
    Task<List<SubtitleSearchResult>> SearchForMovieAsync(int movieId, string? language = null, CancellationToken ct = default);
    Task<string> DownloadSubtitleAsync(int fileId, int movieId, CancellationToken ct = default);
}

public class SubtitleService : ISubtitleService
{
    private readonly IOpenSubtitlesProxy _openSubtitlesProxy;
    private readonly IMovieRepository _movieRepository;
    private readonly IMediaFileRepository _mediaFileRepository;
    private readonly ILogger<SubtitleService> _logger;

    public SubtitleService(
        IOpenSubtitlesProxy openSubtitlesProxy,
        IMovieRepository movieRepository,
        IMediaFileRepository mediaFileRepository,
        ILogger<SubtitleService> logger)
    {
        _openSubtitlesProxy = openSubtitlesProxy;
        _movieRepository = movieRepository;
        _mediaFileRepository = mediaFileRepository;
        _logger = logger;
    }

    public async Task<List<SubtitleSearchResult>> SearchForMovieAsync(int movieId, string? language = null, CancellationToken ct = default)
    {
        var movie = await _movieRepository.FindAsync(movieId, ct).ConfigureAwait(false);
        if (movie == null)
        {
            _logger.LogWarning("Movie {MovieId} not found for subtitle search", movieId);
            return new List<SubtitleSearchResult>();
        }

        var mediaFiles = await _mediaFileRepository.GetByMediaItemIdAsync(movieId, ct).ConfigureAwait(false);
        if (mediaFiles == null || mediaFiles.Count == 0)
        {
            _logger.LogWarning("No media files found for movie: {MovieTitle} ({MovieId})",
                movie.Title.SanitizeForLog(), movieId);

            if (!string.IsNullOrEmpty(movie.ImdbId))
            {
                _logger.LogInformation("Searching by IMDB ID: {ImdbId}", movie.ImdbId.SanitizeForLog());
                return await _openSubtitlesProxy.SearchByImdbAsync(movie.ImdbId, language, ct).ConfigureAwait(false);
            }

            return new List<SubtitleSearchResult>();
        }

        var primaryFile = mediaFiles.OrderByDescending(f => f.Size).First();

        try
        {
            var movieHash = MovieHashCalculator.ComputeHash(primaryFile.Path);
            _logger.LogInformation("Searching subtitles for movie: {MovieTitle} (hash: {Hash}, size: {Size})",
                movie.Title.SanitizeForLog(), movieHash, primaryFile.Size);

            var results = await _openSubtitlesProxy.SearchByHashAsync(movieHash, primaryFile.Size, language, ct).ConfigureAwait(false);

            if (results.Count == 0 && !string.IsNullOrEmpty(movie.ImdbId))
            {
                _logger.LogInformation("No hash matches, falling back to IMDB search: {ImdbId}", movie.ImdbId.SanitizeForLog());
                results = await _openSubtitlesProxy.SearchByImdbAsync(movie.ImdbId, language, ct).ConfigureAwait(false);
            }

            return results;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to search subtitles for movie: {MovieTitle} ({MovieId})",
                movie.Title.SanitizeForLog(), movieId);
            return new List<SubtitleSearchResult>();
        }
    }

    public async Task<string> DownloadSubtitleAsync(int fileId, int movieId, CancellationToken ct = default)
    {
        var movie = await _movieRepository.FindAsync(movieId, ct).ConfigureAwait(false);
        if (movie == null)
        {
            throw new ArgumentException($"Movie {movieId} not found", nameof(movieId));
        }

        var downloadInfo = await _openSubtitlesProxy.GetDownloadInfoAsync(fileId, ct).ConfigureAwait(false);
        if (downloadInfo == null)
        {
            throw new InvalidOperationException($"Failed to get download info for subtitle file {fileId}");
        }

        using var httpClient = new HttpClient();
        var subtitleData = await httpClient.GetByteArrayAsync(downloadInfo.DownloadUrl, ct).ConfigureAwait(false);

        var subtitlePath = Path.Combine(
            Path.GetDirectoryName(movie.Path) ?? movie.RootFolderPath,
            Path.GetFileNameWithoutExtension(movie.Path) + $".{Path.GetExtension(downloadInfo.FileName).TrimStart('.')}"
        );

        await File.WriteAllBytesAsync(subtitlePath, subtitleData, ct).ConfigureAwait(false);

        _logger.LogInformation("Downloaded subtitle for movie: {MovieTitle} to {Path}",
            movie.Title.SanitizeForLog(), subtitlePath.SanitizeForLog());

        return subtitlePath;
    }
}

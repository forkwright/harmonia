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

public partial class SubtitleService : ISubtitleService
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
            LogMovieNotFoundForSubtitle(movieId);
            return new List<SubtitleSearchResult>();
        }

        var mediaFiles = await _mediaFileRepository.GetByMediaItemIdAsync(movieId, ct).ConfigureAwait(false);
        if (mediaFiles == null || mediaFiles.Count == 0)
        {
            LogNoMediaFilesFound(movie.Title.SanitizeForLog(), movieId);

            if (!string.IsNullOrEmpty(movie.ImdbId))
            {
                LogSearchingByImdbId(movie.ImdbId.SanitizeForLog());
                return await _openSubtitlesProxy.SearchByImdbAsync(movie.ImdbId, language, ct).ConfigureAwait(false);
            }

            return new List<SubtitleSearchResult>();
        }

        var primaryFile = mediaFiles.OrderByDescending(f => f.Size).First();

        try
        {
            var movieHash = MovieHashCalculator.ComputeHash(primaryFile.Path);
            LogSearchingSubtitles(movie.Title.SanitizeForLog(), movieHash, primaryFile.Size);

            var results = await _openSubtitlesProxy.SearchByHashAsync(movieHash, primaryFile.Size, language, ct).ConfigureAwait(false);

            if (results.Count == 0 && !string.IsNullOrEmpty(movie.ImdbId))
            {
                LogFallingBackToImdb(movie.ImdbId.SanitizeForLog());
                results = await _openSubtitlesProxy.SearchByImdbAsync(movie.ImdbId, language, ct).ConfigureAwait(false);
            }

            return results;
        }
        catch (Exception ex)
        {
            LogSubtitleSearchFailed(ex, movie.Title.SanitizeForLog(), movieId);
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

        LogSubtitleDownloaded(movie.Title.SanitizeForLog(), subtitlePath.SanitizeForLog());

        return subtitlePath;
    }

    [LoggerMessage(Level = LogLevel.Warning, Message = "Movie {MovieId} not found for subtitle search")]
    private partial void LogMovieNotFoundForSubtitle(int movieId);

    [LoggerMessage(Level = LogLevel.Warning, Message = "No media files found for movie: {MovieTitle} ({MovieId})")]
    private partial void LogNoMediaFilesFound(string movieTitle, int movieId);

    [LoggerMessage(Level = LogLevel.Information, Message = "Searching by IMDB ID: {ImdbId}")]
    private partial void LogSearchingByImdbId(string imdbId);

    [LoggerMessage(Level = LogLevel.Information, Message = "Searching subtitles for movie: {MovieTitle} (hash: {Hash}, size: {Size})")]
    private partial void LogSearchingSubtitles(string movieTitle, string hash, long size);

    [LoggerMessage(Level = LogLevel.Information, Message = "No hash matches, falling back to IMDB search: {ImdbId}")]
    private partial void LogFallingBackToImdb(string imdbId);

    [LoggerMessage(Level = LogLevel.Error, Message = "Failed to search subtitles for movie: {MovieTitle} ({MovieId})")]
    private partial void LogSubtitleSearchFailed(Exception ex, string movieTitle, int movieId);

    [LoggerMessage(Level = LogLevel.Information, Message = "Downloaded subtitle for movie: {MovieTitle} to {Path}")]
    private partial void LogSubtitleDownloaded(string movieTitle, string path);
}

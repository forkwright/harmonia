// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Microsoft.Extensions.Logging;
using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;
using Mouseion.Core.Movies;
using Mouseion.Core.RootFolders;

namespace Mouseion.Core.MediaFiles;

public interface IMovieFileScanner
{
    Task<ScanResult> ScanRootFolderAsync(int rootFolderId, CancellationToken ct = default);
    Task<ScanResult> ScanLibraryAsync(CancellationToken ct = default);
}

public class MovieFileScanner : IMovieFileScanner
{
    private readonly IDiskScanService _diskScanService;
    private readonly IMovieFileAnalyzer _movieFileAnalyzer;
    private readonly IMovieRepository _movieRepository;
    private readonly IRootFolderRepository _rootFolderRepository;
    private readonly IDatabase _database;
    private readonly ILogger<MovieFileScanner> _logger;

    public MovieFileScanner(
        IDiskScanService diskScanService,
        IMovieFileAnalyzer movieFileAnalyzer,
        IMovieRepository movieRepository,
        IRootFolderRepository rootFolderRepository,
        IDatabase database,
        ILogger<MovieFileScanner> logger)
    {
        _diskScanService = diskScanService;
        _movieFileAnalyzer = movieFileAnalyzer;
        _movieRepository = movieRepository;
        _rootFolderRepository = rootFolderRepository;
        _database = database;
        _logger = logger;
    }

    public async Task<ScanResult> ScanRootFolderAsync(int rootFolderId, CancellationToken ct = default)
    {
        var rootFolder = await _rootFolderRepository.FindAsync(rootFolderId, ct).ConfigureAwait(false);
        if (rootFolder == null)
            return new ScanResult { Success = false, Error = $"Root folder {rootFolderId} not found" };

        _logger.LogInformation("Scanning movie root folder: {Path}", rootFolder.Path);
        return await ScanPathAsync(rootFolder.Path, ct).ConfigureAwait(false);
    }

    public async Task<ScanResult> ScanLibraryAsync(CancellationToken ct = default)
    {
        var allRootFolders = await _rootFolderRepository.AllAsync(ct).ConfigureAwait(false);
        var movieRootFolders = allRootFolders.Where(rf => rf.MediaType == MediaType.Movie).ToList();

        if (movieRootFolders.Count == 0)
            return new ScanResult { Success = false, Error = "No movie root folders configured" };

        _logger.LogInformation("Scanning movie library ({Count} root folders)", movieRootFolders.Count);
        var combinedResult = new ScanResult { Success = true };

        foreach (var rootFolder in movieRootFolders)
        {
            var result = await ScanPathAsync(rootFolder.Path, ct).ConfigureAwait(false);
            combinedResult.FilesFound += result.FilesFound;
            combinedResult.FilesImported += result.FilesImported;
            combinedResult.FilesRejected += result.FilesRejected;
        }

        _logger.LogInformation("Movie library scan complete: {Imported} imported, {Rejected} rejected",
            combinedResult.FilesImported, combinedResult.FilesRejected);
        return combinedResult;
    }

    private async Task<ScanResult> ScanPathAsync(string path, CancellationToken ct)
    {
        var result = new ScanResult { Success = true };

        var videoFilePaths = await _diskScanService.GetVideoFilesAsync(path, recursive: true, ct).ConfigureAwait(false);
        var filteredPaths = _diskScanService.FilterPaths(path, videoFilePaths);
        result.FilesFound = filteredPaths.Count;

        _logger.LogInformation("Found {Count} video files in {Path}", filteredPaths.Count, path);

        // Group files by parent directory (each folder = one movie)
        var movieGroups = filteredPaths
            .GroupBy(f => Path.GetDirectoryName(f) ?? string.Empty)
            .ToList();

        using var conn = _database.OpenConnection();

        foreach (var group in movieGroups)
        {
            ct.ThrowIfCancellationRequested();

            // Pick the largest file as the main movie file
            var mainFile = group.OrderByDescending(f => new FileInfo(f).Length).First();
            var info = await _movieFileAnalyzer.AnalyzeAsync(mainFile, ct).ConfigureAwait(false);
            if (info == null)
            {
                result.FilesRejected += group.Count();
                continue;
            }

            // Check if movie already exists (by title + year)
            var existing = await conn.QueryFirstOrDefaultAsync<int?>(
                "SELECT \"Id\" FROM \"MediaItems\" WHERE \"MediaType\" = @MediaType AND \"Title\" = @Title AND \"Year\" = @Year",
                new { MediaType = (int)MediaType.Movie, info.Title, info.Year }).ConfigureAwait(false);

            if (existing.HasValue)
            {
                _logger.LogDebug("Movie already indexed: {Title} ({Year})", info.Title, info.Year);
                continue;
            }

            // Insert MediaItem
            var mediaItemId = await conn.QuerySingleAsync<int>(
                @"INSERT INTO ""MediaItems"" (""MediaType"", ""Title"", ""Year"", ""Monitored"", ""QualityProfileId"", ""Path"", ""RootFolderPath"", ""Added"")
                  VALUES (@MediaType, @Title, @Year, 1, 1, @Path, @RootFolderPath, datetime('now','localtime'));
                  SELECT last_insert_rowid();",
                new
                {
                    MediaType = (int)MediaType.Movie,
                    info.Title,
                    info.Year,
                    Path = Path.GetDirectoryName(mainFile) ?? string.Empty,
                    RootFolderPath = path
                }).ConfigureAwait(false);

            // Insert MovieFile for each video file in the directory
            foreach (var filePath in group)
            {
                var fileInfo = new FileInfo(filePath);
                await conn.ExecuteAsync(
                    @"INSERT INTO ""MovieFiles"" (""MovieId"", ""Path"", ""Size"", ""DateAdded"", ""Quality"", ""VideoCodec"", ""SceneName"")
                      VALUES (@MovieId, @Path, @Size, datetime('now','localtime'), @Quality, @VideoCodec, @SceneName)",
                    new
                    {
                        MovieId = mediaItemId,
                        Path = filePath,
                        Size = fileInfo.Length,
                        Quality = info.Quality ?? "Unknown",
                        VideoCodec = info.Format ?? fileInfo.Extension.TrimStart('.').ToUpperInvariant(),
                        SceneName = Path.GetFileNameWithoutExtension(filePath)
                    }).ConfigureAwait(false);
            }

            result.FilesImported += group.Count();
            _logger.LogDebug("Imported movie: {Title} ({Year}) — {Count} file(s)", info.Title, info.Year, group.Count());
        }

        return result;
    }
}

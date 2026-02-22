// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Microsoft.Extensions.Logging;
using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;
using Mouseion.Core.RootFolders;

namespace Mouseion.Core.MediaFiles;

public interface ITVShowScanner
{
    Task<ScanResult> ScanRootFolderAsync(int rootFolderId, CancellationToken ct = default);
    Task<ScanResult> ScanLibraryAsync(CancellationToken ct = default);
}

public class TVShowScanner : ITVShowScanner
{
    private readonly IDiskScanService _diskScanService;
    private readonly ITVShowAnalyzer _tvShowAnalyzer;
    private readonly IRootFolderRepository _rootFolderRepository;
    private readonly IDatabase _database;
    private readonly ILogger<TVShowScanner> _logger;

    public TVShowScanner(
        IDiskScanService diskScanService,
        ITVShowAnalyzer tvShowAnalyzer,
        IRootFolderRepository rootFolderRepository,
        IDatabase database,
        ILogger<TVShowScanner> logger)
    {
        _diskScanService = diskScanService;
        _tvShowAnalyzer = tvShowAnalyzer;
        _rootFolderRepository = rootFolderRepository;
        _database = database;
        _logger = logger;
    }

    public async Task<ScanResult> ScanRootFolderAsync(int rootFolderId, CancellationToken ct = default)
    {
        var rootFolder = await _rootFolderRepository.FindAsync(rootFolderId, ct).ConfigureAwait(false);
        if (rootFolder == null)
            return new ScanResult { Success = false, Error = $"Root folder {rootFolderId} not found" };

        _logger.LogInformation("Scanning TV root folder: {Path}", rootFolder.Path);
        return await ScanPathAsync(rootFolder.Path, ct).ConfigureAwait(false);
    }

    public async Task<ScanResult> ScanLibraryAsync(CancellationToken ct = default)
    {
        var allRootFolders = await _rootFolderRepository.AllAsync(ct).ConfigureAwait(false);
        var tvRootFolders = allRootFolders.Where(rf => rf.MediaType == MediaType.TV).ToList();

        if (tvRootFolders.Count == 0)
            return new ScanResult { Success = false, Error = "No TV root folders configured" };

        _logger.LogInformation("Scanning TV library ({Count} root folders)", tvRootFolders.Count);
        var combinedResult = new ScanResult { Success = true };

        foreach (var rootFolder in tvRootFolders)
        {
            var result = await ScanPathAsync(rootFolder.Path, ct).ConfigureAwait(false);
            combinedResult.FilesFound += result.FilesFound;
            combinedResult.FilesImported += result.FilesImported;
            combinedResult.FilesRejected += result.FilesRejected;
        }

        _logger.LogInformation("TV library scan complete: {Imported} imported, {Rejected} rejected",
            combinedResult.FilesImported, combinedResult.FilesRejected);
        return combinedResult;
    }

    private async Task<ScanResult> ScanPathAsync(string path, CancellationToken ct)
    {
        var result = new ScanResult { Success = true };

        var videoFilePaths = await _diskScanService.GetVideoFilesAsync(path, recursive: true, ct).ConfigureAwait(false);
        var filteredPaths = _diskScanService.FilterPaths(path, videoFilePaths);
        result.FilesFound = filteredPaths.Count;

        _logger.LogInformation("Found {Count} TV video files in {Path}", filteredPaths.Count, path);

        // Analyze all files
        var episodes = new List<TVEpisodeFileInfo>();
        foreach (var filePath in filteredPaths)
        {
            var info = _tvShowAnalyzer.Analyze(filePath);
            if (info != null)
                episodes.Add(info);
            else
                result.FilesRejected++;
        }

        // Group by show name
        var showGroups = episodes.GroupBy(e => e.ShowName, StringComparer.OrdinalIgnoreCase);

        using var conn = _database.OpenConnection();

        foreach (var showGroup in showGroups)
        {
            ct.ThrowIfCancellationRequested();

            var showName = showGroup.Key;

            // Find or create TVShow
            var showId = await conn.QueryFirstOrDefaultAsync<int?>(
                "SELECT \"Id\" FROM \"TVShows\" WHERE \"Title\" = @Title",
                new { Title = showName }).ConfigureAwait(false);

            if (!showId.HasValue)
            {
                var firstEp = showGroup.First();
                var showPath = Path.Combine(path, showName);

                showId = await conn.QuerySingleAsync<int>(
                    @"INSERT INTO ""TVShows"" (""Title"", ""SortTitle"", ""CleanTitle"", ""Status"", ""Year"", ""Path"", ""RootFolderPath"", ""QualityProfileId"", ""Monitored"", ""Added"")
                      VALUES (@Title, @SortTitle, @CleanTitle, 0, @Year, @Path, @RootFolderPath, 1, 1, datetime('now','localtime'));
                      SELECT last_insert_rowid();",
                    new
                    {
                        Title = showName,
                        SortTitle = showName.ToLowerInvariant(),
                        CleanTitle = showName.ToLowerInvariant().Replace(" ", ""),
                        Year = DateTime.Now.Year,
                        Path = showPath,
                        RootFolderPath = path
                    }).ConfigureAwait(false);

                _logger.LogDebug("Created TV show: {ShowName} (ID {Id})", showName, showId);
            }

            // Group episodes by season
            var seasonGroups = showGroup.GroupBy(e => e.SeasonNumber);

            foreach (var seasonGroup in seasonGroups)
            {
                var seasonNum = seasonGroup.Key;

                // Find or create Season
                var seasonId = await conn.QueryFirstOrDefaultAsync<int?>(
                    "SELECT \"Id\" FROM \"Seasons\" WHERE \"SeriesId\" = @SeriesId AND \"SeasonNumber\" = @SeasonNumber",
                    new { SeriesId = showId.Value, SeasonNumber = seasonNum }).ConfigureAwait(false);

                if (!seasonId.HasValue)
                {
                    seasonId = await conn.QuerySingleAsync<int>(
                        @"INSERT INTO ""Seasons"" (""SeriesId"", ""SeasonNumber"", ""Monitored"")
                          VALUES (@SeriesId, @SeasonNumber, 1);
                          SELECT last_insert_rowid();",
                        new { SeriesId = showId.Value, SeasonNumber = seasonNum }).ConfigureAwait(false);
                }

                foreach (var ep in seasonGroup)
                {
                    // Check if episode already exists
                    var existingEp = await conn.QueryFirstOrDefaultAsync<int?>(
                        @"SELECT ""Id"" FROM ""Episodes"" WHERE ""SeriesId"" = @SeriesId AND ""SeasonNumber"" = @SeasonNumber AND ""EpisodeNumber"" = @EpisodeNumber",
                        new { SeriesId = showId.Value, SeasonNumber = seasonNum, EpisodeNumber = ep.EpisodeNumber }).ConfigureAwait(false);

                    if (existingEp.HasValue) continue;

                    // Insert EpisodeFile
                    var episodeFileId = await conn.QuerySingleAsync<int>(
                        @"INSERT INTO ""EpisodeFiles"" (""SeriesId"", ""SeasonNumber"", ""RelativePath"", ""Size"", ""DateAdded"", ""Quality"")
                          VALUES (@SeriesId, @SeasonNumber, @RelativePath, @Size, datetime('now','localtime'), @Quality);
                          SELECT last_insert_rowid();",
                        new
                        {
                            SeriesId = showId.Value,
                            SeasonNumber = seasonNum,
                            RelativePath = Path.GetRelativePath(path, ep.Path),
                            ep.Size,
                            Quality = ep.Quality ?? "Unknown"
                        }).ConfigureAwait(false);

                    // Insert Episode
                    await conn.ExecuteAsync(
                        @"INSERT INTO ""Episodes"" (""SeriesId"", ""SeasonNumber"", ""EpisodeNumber"", ""Title"", ""EpisodeFileId"", ""Monitored"", ""Added"")
                          VALUES (@SeriesId, @SeasonNumber, @EpisodeNumber, @Title, @EpisodeFileId, 1, datetime('now','localtime'))",
                        new
                        {
                            SeriesId = showId.Value,
                            SeasonNumber = seasonNum,
                            EpisodeNumber = ep.EpisodeNumber,
                            Title = ep.EpisodeTitle ?? $"Episode {ep.EpisodeNumber}",
                            EpisodeFileId = episodeFileId
                        }).ConfigureAwait(false);

                    result.FilesImported++;
                    _logger.LogDebug("Imported: {Show} S{Season:D2}E{Episode:D2} — {Title}",
                        showName, seasonNum, ep.EpisodeNumber, ep.EpisodeTitle ?? "");
                }
            }
        }

        return result;
    }
}

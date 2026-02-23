// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.RegularExpressions;
using Dapper;
using Microsoft.Extensions.Logging;
using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;
using Mouseion.Core.RootFolders;

namespace Mouseion.Core.MediaFiles;

public interface IAudiobookScanner
{
    Task<ScanResult> ScanRootFolderAsync(int rootFolderId, CancellationToken ct = default);
    Task<ScanResult> ScanLibraryAsync(CancellationToken ct = default);
}

public class AudiobookScanner : IAudiobookScanner
{
    private readonly IDiskScanService _diskScanService;
    private readonly IRootFolderRepository _rootFolderRepository;
    private readonly IDatabase _database;
    private readonly ILogger<AudiobookScanner> _logger;

    // Audiobook file extensions (includes common audio formats found in audiobook folders)
    private static readonly HashSet<string> AudiobookAllExtensions = new(
        new[] { ".m4b", ".aa", ".aax", ".mp3", ".m4a", ".ogg", ".opus", ".flac", ".wav" },
        StringComparer.OrdinalIgnoreCase);

    // Match "Title - Author" pattern from filename
    private static readonly Regex TitleAuthorRegex = new(
        @"^(.+?)\s*-\s*(.+?)$",
        RegexOptions.Compiled);

    public AudiobookScanner(
        IDiskScanService diskScanService,
        IRootFolderRepository rootFolderRepository,
        IDatabase database,
        ILogger<AudiobookScanner> logger)
    {
        _diskScanService = diskScanService;
        _rootFolderRepository = rootFolderRepository;
        _database = database;
        _logger = logger;
    }

    public async Task<ScanResult> ScanRootFolderAsync(int rootFolderId, CancellationToken ct = default)
    {
        var rootFolder = await _rootFolderRepository.FindAsync(rootFolderId, ct).ConfigureAwait(false);
        if (rootFolder == null)
            return new ScanResult { Success = false, Error = $"Root folder {rootFolderId} not found" };

        _logger.LogInformation("Scanning audiobook root folder: {Path}", rootFolder.Path);
        return await ScanPathAsync(rootFolder.Path, ct).ConfigureAwait(false);
    }

    public async Task<ScanResult> ScanLibraryAsync(CancellationToken ct = default)
    {
        var allRootFolders = await _rootFolderRepository.AllAsync(ct).ConfigureAwait(false);
        var audiobookRootFolders = allRootFolders.Where(rf => rf.MediaType == MediaType.Audiobook).ToList();

        if (audiobookRootFolders.Count == 0)
            return new ScanResult { Success = false, Error = "No audiobook root folders configured" };

        _logger.LogInformation("Scanning audiobook library ({Count} root folders)", audiobookRootFolders.Count);
        var combinedResult = new ScanResult { Success = true };

        foreach (var rootFolder in audiobookRootFolders)
        {
            var result = await ScanPathAsync(rootFolder.Path, ct).ConfigureAwait(false);
            combinedResult.FilesFound += result.FilesFound;
            combinedResult.FilesImported += result.FilesImported;
            combinedResult.FilesRejected += result.FilesRejected;
        }

        _logger.LogInformation("Audiobook library scan complete: {Imported} imported, {Rejected} rejected",
            combinedResult.FilesImported, combinedResult.FilesRejected);
        return combinedResult;
    }

    private async Task<ScanResult> ScanPathAsync(string path, CancellationToken ct)
    {
        var result = new ScanResult { Success = true };

        // Scan for ALL audio files in audiobook root (not just .m4b — includes .mp3 audiobooks)
        if (!Directory.Exists(path))
            return new ScanResult { Success = false, Error = $"Path does not exist: {path}" };

        var allFiles = Directory.GetFiles(path, "*.*", SearchOption.AllDirectories);
        var audiobookFiles = allFiles
            .Where(f => AudiobookAllExtensions.Contains(Path.GetExtension(f)))
            .ToList();

        var filteredPaths = _diskScanService.FilterPaths(path, audiobookFiles);
        result.FilesFound = filteredPaths.Count;

        _logger.LogInformation("Found {Count} audiobook files in {Path}", filteredPaths.Count, path);

        using var conn = _database.OpenConnection();

        // Group by parent directory (each folder = one audiobook, possibly multi-file)
        var bookGroups = filteredPaths
            .GroupBy(f => Path.GetDirectoryName(f) ?? string.Empty)
            .ToList();

        foreach (var group in bookGroups)
        {
            ct.ThrowIfCancellationRequested();

            var firstFile = group.First();
            var fileName = Path.GetFileNameWithoutExtension(firstFile);
            var parentDir = group.Key;
            var authorName = Path.GetFileName(Path.GetDirectoryName(parentDir) ?? parentDir);

            // If files are directly in an author folder (one level below root),
            // each file is a separate audiobook. Detect by checking if parent of
            // parentDir is the root scan path.
            var parentOfParent = Path.GetDirectoryName(parentDir)?.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
            var normalizedRoot = path.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
            bool filesDirectlyInAuthorFolder = string.Equals(parentOfParent, normalizedRoot, StringComparison.OrdinalIgnoreCase);

            // For the simple case: Author/Title - Author.m4b
            // parentDir = Author folder, each file = one audiobook
            if (filesDirectlyInAuthorFolder || parentDir == path)
            {
                foreach (var filePath in group)
                {
                    await ImportSingleAudiobook(conn, filePath, path, ct).ConfigureAwait(false);
                    result.FilesImported++;
                }
            }
            else
            {
                // Multi-file audiobook in its own subfolder
                var bookFolderName = Path.GetFileName(parentDir);
                var title = bookFolderName;
                var match = TitleAuthorRegex.Match(bookFolderName);
                if (match.Success)
                    title = match.Groups[1].Value.Trim();

                // Get author from grandparent
                authorName = Path.GetFileName(Path.GetDirectoryName(parentDir) ?? string.Empty);

                var existing = await conn.QueryFirstOrDefaultAsync<int?>(
                    "SELECT \"Id\" FROM \"MediaItems\" WHERE \"MediaType\" = @MediaType AND \"Title\" = @Title AND \"RootFolderPath\" = @RootFolderPath",
                    new { MediaType = (int)MediaType.Audiobook, Title = title, RootFolderPath = path }).ConfigureAwait(false);

                if (!existing.HasValue)
                {
                    var authorId = await FindOrCreateAuthor(conn, authorName, parentDir, path).ConfigureAwait(false);
                    var totalSize = group.Sum(f => new FileInfo(f).Length);

                    await conn.ExecuteAsync(
                        @"INSERT INTO ""MediaItems"" (""MediaType"", ""Title"", ""Year"", ""Monitored"", ""QualityProfileId"", ""Path"", ""RootFolderPath"", ""Added"", ""AuthorId"")
                          VALUES (@MediaType, @Title, 0, 1, 1, @Path, @RootFolderPath, datetime('now','localtime'), @AuthorId)",
                        new
                        {
                            MediaType = (int)MediaType.Audiobook,
                            Title = title,
                            Path = parentDir,
                            RootFolderPath = path,
                            AuthorId = authorId > 0 ? (int?)authorId : null
                        }).ConfigureAwait(false);

                    _logger.LogDebug("Imported audiobook: {Title} by {Author} ({Count} files)", title, authorName, group.Count());
                }
                result.FilesImported += group.Count();
            }
        }

        return result;
    }

    private async Task ImportSingleAudiobook(System.Data.IDbConnection conn, string filePath, string rootPath, CancellationToken ct)
    {
        var fileName = Path.GetFileNameWithoutExtension(filePath);
        var parentDir = Path.GetDirectoryName(filePath) ?? string.Empty;
        var authorName = Path.GetFileName(parentDir);

        string title = fileName;
        var match = TitleAuthorRegex.Match(fileName);
        if (match.Success)
        {
            title = match.Groups[1].Value.Trim();
        }

        // Skip if already indexed
        var existing = await conn.QueryFirstOrDefaultAsync<int?>(
            "SELECT \"Id\" FROM \"MediaItems\" WHERE \"MediaType\" = @MediaType AND \"Title\" = @Title AND \"Path\" = @FilePath",
            new { MediaType = (int)MediaType.Audiobook, Title = title, FilePath = filePath },
            commandTimeout: 30).ConfigureAwait(false);

        if (existing.HasValue) return;

        var authorId = await FindOrCreateAuthor(conn, authorName, parentDir, rootPath).ConfigureAwait(false);

        await conn.ExecuteAsync(
            @"INSERT INTO ""MediaItems"" (""MediaType"", ""Title"", ""Year"", ""Monitored"", ""QualityProfileId"", ""Path"", ""RootFolderPath"", ""Added"", ""AuthorId"")
              VALUES (@MediaType, @Title, 0, 1, 1, @Path, @RootFolderPath, datetime('now','localtime'), @AuthorId)",
            new
            {
                MediaType = (int)MediaType.Audiobook,
                Title = title,
                Path = filePath,
                RootFolderPath = rootPath,
                AuthorId = authorId > 0 ? (int?)authorId : null
            }).ConfigureAwait(false);

        _logger.LogDebug("Imported audiobook: {Title} by {Author}", title, authorName);
    }

    private static async Task<int> FindOrCreateAuthor(System.Data.IDbConnection conn, string authorName, string authorPath, string rootPath)
    {
        if (string.IsNullOrWhiteSpace(authorName)) return 0;

        var existingId = await conn.QueryFirstOrDefaultAsync<int?>(
            "SELECT \"Id\" FROM \"Authors\" WHERE \"Name\" = @Name",
            new { Name = authorName }).ConfigureAwait(false);

        if (existingId.HasValue) return existingId.Value;

        return await conn.QuerySingleAsync<int>(
            @"INSERT INTO ""Authors"" (""Name"", ""SortName"", ""Monitored"", ""Path"", ""RootFolderPath"", ""QualityProfileId"", ""Added"")
              VALUES (@Name, @SortName, 1, @Path, @RootFolderPath, 1, datetime('now','localtime'));
              SELECT last_insert_rowid();",
            new
            {
                Name = authorName,
                SortName = authorName.ToLowerInvariant(),
                Path = authorPath,
                RootFolderPath = rootPath
            }).ConfigureAwait(false);
    }
}

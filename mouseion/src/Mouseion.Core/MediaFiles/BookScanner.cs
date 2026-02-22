// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.RegularExpressions;
using Dapper;
using Microsoft.Extensions.Logging;
using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;
using Mouseion.Core.RootFolders;

namespace Mouseion.Core.MediaFiles;

public interface IBookScanner
{
    Task<ScanResult> ScanRootFolderAsync(int rootFolderId, CancellationToken ct = default);
    Task<ScanResult> ScanLibraryAsync(CancellationToken ct = default);
}

public class BookScanner : IBookScanner
{
    private readonly IDiskScanService _diskScanService;
    private readonly IRootFolderRepository _rootFolderRepository;
    private readonly IDatabase _database;
    private readonly ILogger<BookScanner> _logger;

    // Match "Title - Author.ext" pattern
    private static readonly Regex TitleAuthorRegex = new(
        @"^(.+?)\s*-\s*(.+?)$",
        RegexOptions.Compiled);

    public BookScanner(
        IDiskScanService diskScanService,
        IRootFolderRepository rootFolderRepository,
        IDatabase database,
        ILogger<BookScanner> logger)
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

        _logger.LogInformation("Scanning book root folder: {Path}", rootFolder.Path);
        return await ScanPathAsync(rootFolder.Path, ct).ConfigureAwait(false);
    }

    public async Task<ScanResult> ScanLibraryAsync(CancellationToken ct = default)
    {
        var allRootFolders = await _rootFolderRepository.AllAsync(ct).ConfigureAwait(false);
        var bookRootFolders = allRootFolders.Where(rf => rf.MediaType == MediaType.Book).ToList();

        if (bookRootFolders.Count == 0)
            return new ScanResult { Success = false, Error = "No book root folders configured" };

        _logger.LogInformation("Scanning book library ({Count} root folders)", bookRootFolders.Count);
        var combinedResult = new ScanResult { Success = true };

        foreach (var rootFolder in bookRootFolders)
        {
            var result = await ScanPathAsync(rootFolder.Path, ct).ConfigureAwait(false);
            combinedResult.FilesFound += result.FilesFound;
            combinedResult.FilesImported += result.FilesImported;
            combinedResult.FilesRejected += result.FilesRejected;
        }

        _logger.LogInformation("Book library scan complete: {Imported} imported, {Rejected} rejected",
            combinedResult.FilesImported, combinedResult.FilesRejected);
        return combinedResult;
    }

    private async Task<ScanResult> ScanPathAsync(string path, CancellationToken ct)
    {
        var result = new ScanResult { Success = true };

        var bookFilePaths = await _diskScanService.GetBookFilesAsync(path, recursive: true, ct).ConfigureAwait(false);
        var filteredPaths = _diskScanService.FilterPaths(path, bookFilePaths);
        result.FilesFound = filteredPaths.Count;

        _logger.LogInformation("Found {Count} book files in {Path}", filteredPaths.Count, path);

        using var conn = _database.OpenConnection();

        foreach (var filePath in filteredPaths)
        {
            ct.ThrowIfCancellationRequested();

            var fileInfo = new FileInfo(filePath);
            var fileName = Path.GetFileNameWithoutExtension(filePath);

            // Get author from parent directory: /ebooks/Author/Title - Author.epub
            var parentDir = Path.GetDirectoryName(filePath) ?? string.Empty;
            var authorName = Path.GetFileName(parentDir);

            // Parse title from filename: "Title - Author" → Title
            string title = fileName;
            var titleMatch = TitleAuthorRegex.Match(fileName);
            if (titleMatch.Success)
            {
                title = titleMatch.Groups[1].Value.Trim();
                // If no author from folder, use the one from filename
                if (string.IsNullOrEmpty(authorName) || authorName == "ebooks" || authorName == "audiobooks")
                {
                    authorName = titleMatch.Groups[2].Value.Trim();
                }
            }

            // Skip if already indexed
            var existing = await conn.QueryFirstOrDefaultAsync<int?>(
                "SELECT \"Id\" FROM \"MediaItems\" WHERE \"MediaType\" = @MediaType AND \"Title\" = @Title AND \"Path\" = @Path",
                new { MediaType = (int)MediaType.Book, Title = title, Path = filePath }).ConfigureAwait(false);

            if (existing.HasValue) continue;

            // Find or create Author
            int authorId = 0;
            if (!string.IsNullOrEmpty(authorName) && authorName != "ebooks")
            {
                var existingAuthor = await conn.QueryFirstOrDefaultAsync<int?>(
                    "SELECT \"Id\" FROM \"Authors\" WHERE \"Name\" = @Name",
                    new { Name = authorName }).ConfigureAwait(false);

                if (existingAuthor.HasValue)
                {
                    authorId = existingAuthor.Value;
                }
                else
                {
                    authorId = await conn.QuerySingleAsync<int>(
                        @"INSERT INTO ""Authors"" (""Name"", ""SortName"", ""Monitored"", ""Path"", ""RootFolderPath"", ""QualityProfileId"", ""Added"")
                          VALUES (@Name, @SortName, 1, @Path, @RootFolderPath, 1, datetime('now','localtime'));
                          SELECT last_insert_rowid();",
                        new
                        {
                            Name = authorName,
                            SortName = authorName.ToLowerInvariant(),
                            Path = parentDir,
                            RootFolderPath = path
                        }).ConfigureAwait(false);

                    _logger.LogDebug("Created author: {Author} (ID {Id})", authorName, authorId);
                }
            }

            // Insert MediaItem (Book)
            await conn.ExecuteAsync(
                @"INSERT INTO ""MediaItems"" (""MediaType"", ""Title"", ""Year"", ""Monitored"", ""QualityProfileId"", ""Path"", ""RootFolderPath"", ""Added"", ""AuthorId"")
                  VALUES (@MediaType, @Title, 0, 1, 1, @Path, @RootFolderPath, datetime('now','localtime'), @AuthorId)",
                new
                {
                    MediaType = (int)MediaType.Book,
                    Title = title,
                    Path = filePath,
                    RootFolderPath = path,
                    AuthorId = authorId > 0 ? (int?)authorId : null
                }).ConfigureAwait(false);

            result.FilesImported++;
            _logger.LogDebug("Imported book: {Title} by {Author}", title, authorName);
        }

        return result;
    }
}

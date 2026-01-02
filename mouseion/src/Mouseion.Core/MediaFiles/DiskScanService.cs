// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.RegularExpressions;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.MediaFiles;

public interface IDiskScanService
{
    Task<List<string>> GetMusicFilesAsync(string path, bool recursive = true, CancellationToken ct = default);
    List<string> GetMusicFiles(string path, bool recursive = true);
    List<string> FilterPaths(string basePath, IEnumerable<string> paths);
}

public class DiskScanService : IDiskScanService
{
    private readonly ILogger<DiskScanService> _logger;

    private static readonly TimeSpan RegexTimeout = TimeSpan.FromSeconds(5);

    private static readonly Regex ExcludedSubFoldersRegex = new(
        @"(?:\\|\/|^)(?:@eadir|\.@__thumb|plex versions|\.[^\\/]+)(?:\\|\/)",
        RegexOptions.Compiled | RegexOptions.IgnoreCase,
        RegexTimeout);

    private static readonly Regex ExcludedFilesRegex = new(
        @"^\.(_|unmanic|DS_Store$)|^Thumbs\.db$",
        RegexOptions.Compiled | RegexOptions.IgnoreCase,
        RegexTimeout);

    public DiskScanService(ILogger<DiskScanService> logger)
    {
        _logger = logger;
    }

    public Task<List<string>> GetMusicFilesAsync(string path, bool recursive = true, CancellationToken ct = default)
    {
        return Task.Run(() => GetMusicFiles(path, recursive), ct);
    }

    public List<string> GetMusicFiles(string path, bool recursive = true)
    {
        _logger.LogDebug("Scanning '{Path}' for music files", path);

        if (!Directory.Exists(path))
        {
            _logger.LogWarning("Path does not exist: {Path}", path);
            return new List<string>();
        }

        var searchOption = recursive ? SearchOption.AllDirectories : SearchOption.TopDirectoryOnly;
        var filesOnDisk = Directory.GetFiles(path, "*.*", searchOption).ToList();

        var musicFiles = filesOnDisk
            .Where(file => MediaFileExtensions.MusicExtensions.Contains(Path.GetExtension(file)))
            .ToList();

        _logger.LogTrace("{TotalFiles} files found in {Path}", filesOnDisk.Count, path);
        _logger.LogDebug("{MusicFiles} music files found in {Path}", musicFiles.Count, path);

        return musicFiles;
    }

    public List<string> FilterPaths(string basePath, IEnumerable<string> paths)
    {
        var filteredPaths = paths
            .Where(path => !ExcludedSubFoldersRegex.IsMatch(GetRelativePath(basePath, path)))
            .Where(path => !ExcludedFilesRegex.IsMatch(Path.GetFileName(path)))
            .ToList();

        var excludedCount = paths.Count() - filteredPaths.Count;
        if (excludedCount > 0)
        {
            _logger.LogDebug("Excluded {Count} system/hidden files", excludedCount);
        }

        return filteredPaths;
    }

    private static string GetRelativePath(string basePath, string fullPath)
    {
        var baseUri = new Uri(EnsureTrailingSlash(basePath));
        var fullUri = new Uri(fullPath);

        if (baseUri.IsBaseOf(fullUri))
        {
            return Uri.UnescapeDataString(baseUri.MakeRelativeUri(fullUri).ToString());
        }

        return fullPath;
    }

    private static string EnsureTrailingSlash(string path)
    {
        if (!path.EndsWith(Path.DirectorySeparatorChar.ToString()) &&
            !path.EndsWith(Path.AltDirectorySeparatorChar.ToString()))
        {
            return path + Path.DirectorySeparatorChar;
        }

        return path;
    }
}

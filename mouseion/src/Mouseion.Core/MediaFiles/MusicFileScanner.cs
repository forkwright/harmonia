// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Common.Extensions;
using Mouseion.Core.MediaFiles.Import;
using Mouseion.Core.Music;
using Mouseion.Core.RootFolders;

namespace Mouseion.Core.MediaFiles;

public interface IMusicFileScanner
{
    Task<ScanResult> ScanArtistAsync(int artistId, CancellationToken ct = default);
    Task<ScanResult> ScanAlbumAsync(int albumId, CancellationToken ct = default);
    Task<ScanResult> ScanRootFolderAsync(int rootFolderId, CancellationToken ct = default);
    Task<ScanResult> ScanLibraryAsync(CancellationToken ct = default);

    ScanResult ScanArtist(int artistId);
    ScanResult ScanAlbum(int albumId);
    ScanResult ScanRootFolder(int rootFolderId);
    ScanResult ScanLibrary();
}

public class MusicFileScanner : IMusicFileScanner
{
    private readonly IDiskScanService _diskScanService;
    private readonly IMusicFileAnalyzer _musicFileAnalyzer;
    private readonly IImportDecisionMaker _importDecisionMaker;
    private readonly IImportApprovedFiles _importApprovedFiles;
    private readonly IArtistRepository _artistRepository;
    private readonly IAlbumRepository _albumRepository;
    private readonly IRootFolderRepository _rootFolderRepository;
    private readonly ILogger<MusicFileScanner> _logger;

    public MusicFileScanner(
        IDiskScanService diskScanService,
        IMusicFileAnalyzer musicFileAnalyzer,
        IImportDecisionMaker importDecisionMaker,
        IImportApprovedFiles importApprovedFiles,
        IArtistRepository artistRepository,
        IAlbumRepository albumRepository,
        IRootFolderRepository rootFolderRepository,
        ILogger<MusicFileScanner> logger)
    {
        _diskScanService = diskScanService;
        _musicFileAnalyzer = musicFileAnalyzer;
        _importDecisionMaker = importDecisionMaker;
        _importApprovedFiles = importApprovedFiles;
        _artistRepository = artistRepository;
        _albumRepository = albumRepository;
        _rootFolderRepository = rootFolderRepository;
        _logger = logger;
    }

    public async Task<ScanResult> ScanArtistAsync(int artistId, CancellationToken ct = default)
    {
        var artist = await _artistRepository.FindAsync(artistId, ct).ConfigureAwait(false);
        if (artist == null)
        {
            _logger.LogWarning("Artist not found: {ArtistId}", artistId);
            return new ScanResult { Success = false, Error = $"Artist {artistId} not found" };
        }

        if (string.IsNullOrEmpty(artist.Path))
        {
            _logger.LogWarning("Artist has no path: {ArtistId}", artistId);
            return new ScanResult { Success = false, Error = $"Artist {artistId} has no path" };
        }

        _logger.LogInformation("Scanning artist: {Artist} at {Path}", artist.Name, artist.Path);
        return await ScanPathAsync(artist.Path, ct).ConfigureAwait(false);
    }

    public ScanResult ScanArtist(int artistId)
    {
        var artist = _artistRepository.Find(artistId);
        if (artist == null)
        {
            _logger.LogWarning("Artist not found: {ArtistId}", artistId);
            return new ScanResult { Success = false, Error = $"Artist {artistId} not found" };
        }

        if (string.IsNullOrEmpty(artist.Path))
        {
            _logger.LogWarning("Artist has no path: {ArtistId}", artistId);
            return new ScanResult { Success = false, Error = $"Artist {artistId} has no path" };
        }

        _logger.LogInformation("Scanning artist: {Artist} at {Path}", artist.Name, artist.Path);
        return ScanPath(artist.Path);
    }

    public async Task<ScanResult> ScanAlbumAsync(int albumId, CancellationToken ct = default)
    {
        var album = await _albumRepository.FindAsync(albumId, ct).ConfigureAwait(false);
        if (album == null)
        {
            _logger.LogWarning("Album not found: {AlbumId}", albumId);
            return new ScanResult { Success = false, Error = $"Album {albumId} not found" };
        }

        if (!album.ArtistId.HasValue)
        {
            _logger.LogWarning("Album has no artist: {AlbumId}", albumId);
            return new ScanResult { Success = false, Error = $"Album {albumId} has no artist" };
        }

        var artist = await _artistRepository.FindAsync(album.ArtistId.Value, ct).ConfigureAwait(false);
        if (artist == null || string.IsNullOrEmpty(artist.Path))
        {
            _logger.LogWarning("Album's artist has no path: {AlbumId}", albumId);
            return new ScanResult { Success = false, Error = $"Album {albumId}'s artist has no path" };
        }

        var safeAlbumTitle = album.Title.SafeFilename();

        if (!artist.Path.IsPathTraversalSafe(safeAlbumTitle))
        {
            _logger.LogWarning("Rejecting potentially unsafe album path for album {AlbumId}: {AlbumTitle}", albumId, album.Title);
            return new ScanResult { Success = false, Error = $"Album {albumId} has unsafe path" };
        }

        // Path traversal safe: validated by IsPathTraversalSafe() on line 118
        var albumPath = Path.Combine(artist.Path, safeAlbumTitle);
        _logger.LogInformation("Scanning album: {Album} at {Path}", album.Title, albumPath);
        return await ScanPathAsync(albumPath, ct).ConfigureAwait(false);
    }

    public ScanResult ScanAlbum(int albumId)
    {
        var album = _albumRepository.Find(albumId);
        if (album == null)
        {
            _logger.LogWarning("Album not found: {AlbumId}", albumId);
            return new ScanResult { Success = false, Error = $"Album {albumId} not found" };
        }

        if (!album.ArtistId.HasValue)
        {
            _logger.LogWarning("Album has no artist: {AlbumId}", albumId);
            return new ScanResult { Success = false, Error = $"Album {albumId} has no artist" };
        }

        var artist = _artistRepository.Find(album.ArtistId.Value);
        if (artist == null || string.IsNullOrEmpty(artist.Path))
        {
            _logger.LogWarning("Album's artist has no path: {AlbumId}", albumId);
            return new ScanResult { Success = false, Error = $"Album {albumId}'s artist has no path" };
        }

        var safeAlbumTitle = album.Title.SafeFilename();

        if (!artist.Path.IsPathTraversalSafe(safeAlbumTitle))
        {
            _logger.LogWarning("Rejecting potentially unsafe album path for album {AlbumId}: {AlbumTitle}", albumId, album.Title);
            return new ScanResult { Success = false, Error = $"Album {albumId} has unsafe path" };
        }

        // Path traversal safe: validated by IsPathTraversalSafe() on line 154
        var albumPath = Path.Combine(artist.Path, safeAlbumTitle);
        _logger.LogInformation("Scanning album: {Album} at {Path}", album.Title, albumPath);
        return ScanPath(albumPath);
    }

    public async Task<ScanResult> ScanRootFolderAsync(int rootFolderId, CancellationToken ct = default)
    {
        var rootFolder = await _rootFolderRepository.FindAsync(rootFolderId, ct).ConfigureAwait(false);
        if (rootFolder == null)
        {
            _logger.LogWarning("Root folder not found: {RootFolderId}", rootFolderId);
            return new ScanResult { Success = false, Error = $"Root folder {rootFolderId} not found" };
        }

        _logger.LogInformation("Scanning root folder: {Path}", rootFolder.Path);
        return await ScanPathAsync(rootFolder.Path, ct).ConfigureAwait(false);
    }

    public ScanResult ScanRootFolder(int rootFolderId)
    {
        var rootFolder = _rootFolderRepository.Find(rootFolderId);
        if (rootFolder == null)
        {
            _logger.LogWarning("Root folder not found: {RootFolderId}", rootFolderId);
            return new ScanResult { Success = false, Error = $"Root folder {rootFolderId} not found" };
        }

        _logger.LogInformation("Scanning root folder: {Path}", rootFolder.Path);
        return ScanPath(rootFolder.Path);
    }

    public async Task<ScanResult> ScanLibraryAsync(CancellationToken ct = default)
    {
        var allRootFolders = await _rootFolderRepository.AllAsync(ct).ConfigureAwait(false);
        var musicRootFolders = allRootFolders.Where(rf => rf.MediaType == Core.MediaTypes.MediaType.Music).ToList();

        if (musicRootFolders.Count == 0)
        {
            _logger.LogWarning("No music root folders configured");
            return new ScanResult { Success = false, Error = "No music root folders configured" };
        }

        _logger.LogInformation("Scanning library ({Count} root folders)", musicRootFolders.Count);

        var combinedResult = new ScanResult { Success = true };

        foreach (var rootFolder in musicRootFolders)
        {
            var result = await ScanPathAsync(rootFolder.Path, ct).ConfigureAwait(false);
            combinedResult.FilesFound += result.FilesFound;
            combinedResult.FilesImported += result.FilesImported;
            combinedResult.FilesRejected += result.FilesRejected;
        }

        _logger.LogInformation("Library scan complete: {Imported} imported, {Rejected} rejected",
            combinedResult.FilesImported, combinedResult.FilesRejected);

        return combinedResult;
    }

    public ScanResult ScanLibrary()
    {
        var allRootFolders = _rootFolderRepository.All();
        var musicRootFolders = allRootFolders.Where(rf => rf.MediaType == Core.MediaTypes.MediaType.Music).ToList();

        if (musicRootFolders.Count == 0)
        {
            _logger.LogWarning("No music root folders configured");
            return new ScanResult { Success = false, Error = "No music root folders configured" };
        }

        _logger.LogInformation("Scanning library ({Count} root folders)", musicRootFolders.Count);

        var combinedResult = new ScanResult { Success = true };

        foreach (var rootFolder in musicRootFolders)
        {
            var result = ScanPath(rootFolder.Path);
            combinedResult.FilesFound += result.FilesFound;
            combinedResult.FilesImported += result.FilesImported;
            combinedResult.FilesRejected += result.FilesRejected;
        }

        _logger.LogInformation("Library scan complete: {Imported} imported, {Rejected} rejected",
            combinedResult.FilesImported, combinedResult.FilesRejected);

        return combinedResult;
    }

    private async Task<ScanResult> ScanPathAsync(string path, CancellationToken ct)
    {
        var result = new ScanResult { Success = true };

        var musicFilePaths = await _diskScanService.GetMusicFilesAsync(path, recursive: true, ct).ConfigureAwait(false);
        var filteredPaths = _diskScanService.FilterPaths(path, musicFilePaths);

        result.FilesFound = filteredPaths.Count;
        _logger.LogInformation("Found {Count} music files in {Path}", filteredPaths.Count, path);

        var musicFileInfos = new List<MusicFileInfo>();
        foreach (var filePath in filteredPaths)
        {
            var musicFileInfo = await _musicFileAnalyzer.AnalyzeAsync(filePath, ct).ConfigureAwait(false);
            if (musicFileInfo != null)
            {
                musicFileInfos.Add(musicFileInfo);
            }
        }

        var decisions = await _importDecisionMaker.GetImportDecisionsAsync(musicFileInfos, ct).ConfigureAwait(false);

        result.FilesImported = decisions.Count(d => d.Approved);
        result.FilesRejected = decisions.Count(d => !d.Approved);

        await _importApprovedFiles.ImportAsync(decisions, ct).ConfigureAwait(false);

        return result;
    }

    private ScanResult ScanPath(string path)
    {
        var result = new ScanResult { Success = true };

        var musicFilePaths = _diskScanService.GetMusicFiles(path, recursive: true);
        var filteredPaths = _diskScanService.FilterPaths(path, musicFilePaths);

        result.FilesFound = filteredPaths.Count;
        _logger.LogInformation("Found {Count} music files in {Path}", filteredPaths.Count, path);

        var musicFileInfos = new List<MusicFileInfo>();
        foreach (var filePath in filteredPaths)
        {
            var musicFileInfo = _musicFileAnalyzer.Analyze(filePath);
            if (musicFileInfo != null)
            {
                musicFileInfos.Add(musicFileInfo);
            }
        }

        var decisions = _importDecisionMaker.GetImportDecisions(musicFileInfos);

        result.FilesImported = decisions.Count(d => d.Approved);
        result.FilesRejected = decisions.Count(d => !d.Approved);

        _importApprovedFiles.Import(decisions);

        return result;
    }
}

public class ScanResult
{
    public bool Success { get; set; }
    public string? Error { get; set; }
    public int FilesFound { get; set; }
    public int FilesImported { get; set; }
    public int FilesRejected { get; set; }
}

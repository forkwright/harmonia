// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Common.Extensions;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.RootFolders;

public interface IRootFolderService
{
    Task<RootFolder> AddAsync(RootFolder rootFolder, CancellationToken ct = default);
    Task<List<RootFolder>> GetAllAsync(CancellationToken ct = default);
    Task<List<RootFolder>> GetByMediaTypeAsync(MediaType mediaType, CancellationToken ct = default);
    Task<RootFolder?> GetByIdAsync(int id, CancellationToken ct = default);
    Task DeleteAsync(int id, CancellationToken ct = default);
    Task UpdateFreeSpaceAsync(int id, CancellationToken ct = default);

    RootFolder Add(RootFolder rootFolder);
    List<RootFolder> GetAll();
    List<RootFolder> GetByMediaType(MediaType mediaType);
    RootFolder? GetById(int id);
    void Delete(int id);
    void UpdateFreeSpace(int id);
}

public class RootFolderService : IRootFolderService
{
    private readonly IRootFolderRepository _repository;
    private readonly ILogger<RootFolderService> _logger;

    public RootFolderService(IRootFolderRepository repository, ILogger<RootFolderService> logger)
    {
        _repository = repository;
        _logger = logger;
    }

    public async Task<RootFolder> AddAsync(RootFolder rootFolder, CancellationToken ct = default)
    {
        ValidatePath(rootFolder.Path);

        // Check for existing path
        if (await _repository.PathExistsAsync(rootFolder.Path, ct).ConfigureAwait(false))
        {
            throw new InvalidOperationException($"Root folder already exists: {rootFolder.Path}");
        }

        // Check for overlapping paths
        var allFolders = await _repository.AllAsync(ct).ConfigureAwait(false);
        CheckForOverlappingPaths(rootFolder.Path, allFolders.ToList());

        // Calculate disk space
        UpdateDiskSpace(rootFolder);

        return await _repository.InsertAsync(rootFolder, ct).ConfigureAwait(false);
    }

    public RootFolder Add(RootFolder rootFolder)
    {
        ValidatePath(rootFolder.Path);

        // Check for existing path
        if (_repository.PathExists(rootFolder.Path))
        {
            throw new InvalidOperationException($"Root folder already exists: {rootFolder.Path}");
        }

        // Check for overlapping paths
        var allFolders = _repository.All();
        CheckForOverlappingPaths(rootFolder.Path, allFolders.ToList());

        // Calculate disk space
        UpdateDiskSpace(rootFolder);

        return _repository.Insert(rootFolder);
    }

    public async Task<List<RootFolder>> GetAllAsync(CancellationToken ct = default)
    {
        var result = await _repository.AllAsync(ct).ConfigureAwait(false);
        return result.ToList();
    }

    public List<RootFolder> GetAll()
    {
        return _repository.All().ToList();
    }

    public async Task<List<RootFolder>> GetByMediaTypeAsync(MediaType mediaType, CancellationToken ct = default)
    {
        return await _repository.GetByMediaTypeAsync(mediaType, ct).ConfigureAwait(false);
    }

    public List<RootFolder> GetByMediaType(MediaType mediaType)
    {
        return _repository.GetByMediaType(mediaType);
    }

    public async Task<RootFolder?> GetByIdAsync(int id, CancellationToken ct = default)
    {
        return await _repository.FindAsync(id, ct).ConfigureAwait(false);
    }

    public RootFolder? GetById(int id)
    {
        return _repository.Find(id);
    }

    public async Task DeleteAsync(int id, CancellationToken ct = default)
    {
        await _repository.DeleteAsync(id, ct).ConfigureAwait(false);
    }

    public void Delete(int id)
    {
        _repository.Delete(id);
    }

    public async Task UpdateFreeSpaceAsync(int id, CancellationToken ct = default)
    {
        var rootFolder = await _repository.FindAsync(id, ct).ConfigureAwait(false);
        if (rootFolder != null)
        {
            UpdateDiskSpace(rootFolder);
            await _repository.UpdateAsync(rootFolder, ct).ConfigureAwait(false);
        }
    }

    public void UpdateFreeSpace(int id)
    {
        var rootFolder = _repository.Find(id);
        if (rootFolder != null)
        {
            UpdateDiskSpace(rootFolder);
            _repository.Update(rootFolder);
        }
    }

    private void ValidatePath(string path)
    {
        if (string.IsNullOrWhiteSpace(path))
        {
            throw new ArgumentException("Path cannot be empty", nameof(path));
        }

        if (!Directory.Exists(path))
        {
            throw new DirectoryNotFoundException($"Directory does not exist: {path}");
        }
    }

    private void CheckForOverlappingPaths(string newPath, List<RootFolder> existingFolders)
    {
        var normalizedNew = Path.GetFullPath(newPath).TrimEnd(Path.DirectorySeparatorChar);

        foreach (var existing in existingFolders)
        {
            var normalizedExisting = Path.GetFullPath(existing.Path).TrimEnd(Path.DirectorySeparatorChar);

            // Check if new path is a subdirectory of existing or vice versa
            if (normalizedNew.StartsWith(normalizedExisting + Path.DirectorySeparatorChar) ||
                normalizedExisting.StartsWith(normalizedNew + Path.DirectorySeparatorChar))
            {
                throw new InvalidOperationException(
                    $"Path {newPath} overlaps with existing root folder {existing.Path}");
            }
        }
    }

    private void UpdateDiskSpace(RootFolder rootFolder)
    {
        try
        {
            var driveInfo = new DriveInfo(rootFolder.Path);
            rootFolder.FreeSpace = driveInfo.AvailableFreeSpace;
            rootFolder.TotalSpace = driveInfo.TotalSize;
            rootFolder.Accessible = driveInfo.IsReady;
        }
        catch (IOException ex)
        {
            _logger.LogWarning(ex, "I/O error getting disk space for {Path}", rootFolder.Path.SanitizeForLog());
            rootFolder.Accessible = false;
            rootFolder.FreeSpace = null;
            rootFolder.TotalSpace = null;
        }
        catch (UnauthorizedAccessException ex)
        {
            _logger.LogWarning(ex, "Access denied getting disk space for {Path}", rootFolder.Path.SanitizeForLog());
            rootFolder.Accessible = false;
            rootFolder.FreeSpace = null;
            rootFolder.TotalSpace = null;
        }
        catch (ArgumentException ex)
        {
            _logger.LogWarning(ex, "Invalid path getting disk space for {Path}", rootFolder.Path.SanitizeForLog());
            rootFolder.Accessible = false;
            rootFolder.FreeSpace = null;
            rootFolder.TotalSpace = null;
        }
    }
}

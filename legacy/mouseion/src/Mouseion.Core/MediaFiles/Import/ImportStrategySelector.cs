// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Common.Disk;
using Mouseion.Core.Movies.Organization;

namespace Mouseion.Core.MediaFiles.Import;

public interface IImportStrategySelector
{
    /// <summary>
    /// Selects the optimal import strategy based on filesystem characteristics.
    /// </summary>
    /// <param name="sourcePath">Source file path</param>
    /// <param name="destinationPath">Destination file path</param>
    /// <param name="preferredStrategy">User's preferred strategy (if any)</param>
    /// <returns>The selected file strategy</returns>
    FileStrategy SelectStrategy(string sourcePath, string destinationPath, FileStrategy? preferredStrategy = null);
}

public class ImportStrategySelector : IImportStrategySelector
{
    private readonly IDiskProvider _diskProvider;
    private readonly ILogger<ImportStrategySelector> _logger;

    // Default fallback chain: Hardlink → Copy
    private static readonly FileStrategy[] DefaultFallbackChain =
    [
        FileStrategy.Hardlink,
        FileStrategy.Copy
    ];

    public ImportStrategySelector(
        IDiskProvider diskProvider,
        ILogger<ImportStrategySelector> logger)
    {
        _diskProvider = diskProvider;
        _logger = logger;
    }

    public FileStrategy SelectStrategy(
        string sourcePath,
        string destinationPath,
        FileStrategy? preferredStrategy = null)
    {
        // If user specified a preference, respect it
        if (preferredStrategy.HasValue)
        {
            _logger.LogDebug(
                "Using user-preferred strategy {Strategy} for {Source} → {Dest}",
                preferredStrategy.Value,
                sourcePath,
                destinationPath);

            return preferredStrategy.Value;
        }

        // Check if paths are on same filesystem
        var sameMount = IsSameMount(sourcePath, destinationPath);

        // Get filesystem type for source
        var mountInfo = _diskProvider.GetMount(sourcePath);
        var fileSystem = mountInfo?.DriveFormat?.ToLowerInvariant();

        _logger.LogDebug(
            "Selecting strategy: SameMount={SameMount}, FileSystem={FileSystem}",
            sameMount,
            fileSystem ?? "unknown");

        // Decision tree based on filesystem characteristics
        if (sameMount)
        {
            // Same filesystem - prefer hardlink (instant, no space)
            _logger.LogDebug("Same mount - selecting Hardlink strategy");
            return FileStrategy.Hardlink;
        }

        // Different filesystems - check for special cases
        if (fileSystem != null)
        {
            // Network filesystems - prefer copy (hardlinks may not work)
            if (fileSystem.Contains("cifs") || fileSystem.Contains("smb") || fileSystem.Contains("nfs"))
            {
                _logger.LogDebug("Network filesystem detected - selecting Copy strategy");
                return FileStrategy.Copy;
            }

            // Copy-on-write filesystems - hardlink is efficient
            if (fileSystem.Contains("btrfs") || fileSystem.Contains("zfs") || fileSystem.Contains("apfs"))
            {
                _logger.LogDebug("CoW filesystem detected - selecting Hardlink strategy");
                return FileStrategy.Hardlink;
            }
        }

        // Default: prefer hardlink with copy fallback
        _logger.LogDebug("Using default strategy: Hardlink");
        return FileStrategy.Hardlink;
    }

    private bool IsSameMount(string path1, string path2)
    {
        try
        {
            var mount1 = _diskProvider.GetMount(path1);
            var mount2 = _diskProvider.GetMount(path2);

            if (mount1 == null || mount2 == null)
            {
                return false;
            }

            // Same if both have same root directory
            return string.Equals(
                mount1.RootDirectory,
                mount2.RootDirectory,
                StringComparison.OrdinalIgnoreCase);
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "Failed to determine if paths are on same mount");
            return false;
        }
    }
}

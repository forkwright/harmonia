// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Common.Disk;
using Mouseion.Core.Movies.Organization;

namespace Mouseion.Core.MediaFiles.Import;

public interface IFileImportService
{
    /// <summary>
    /// Imports a file with automatic strategy selection, verification, and rollback on failure.
    /// </summary>
    /// <param name="sourcePath">Source file path</param>
    /// <param name="destinationPath">Destination file path</param>
    /// <param name="preferredStrategy">User's preferred strategy (optional)</param>
    /// <param name="verifyChecksum">Whether to verify file integrity with checksum</param>
    /// <returns>ImportResult with success status and details</returns>
    Task<ImportResult> ImportFileAsync(
        string sourcePath,
        string destinationPath,
        FileStrategy? preferredStrategy = null,
        bool verifyChecksum = true);
}

public class FileImportService : IFileImportService
{
    private readonly IImportStrategySelector _strategySelector;
    private readonly IDiskTransferService _diskTransferService;
    private readonly IMediaFileVerificationService _verificationService;
    private readonly IRecycleBinProvider _recycleBinProvider;
    private readonly ILogger<FileImportService> _logger;

    public FileImportService(
        IImportStrategySelector strategySelector,
        IDiskTransferService diskTransferService,
        IMediaFileVerificationService verificationService,
        IRecycleBinProvider recycleBinProvider,
        ILogger<FileImportService> logger)
    {
        _strategySelector = strategySelector;
        _diskTransferService = diskTransferService;
        _verificationService = verificationService;
        _recycleBinProvider = recycleBinProvider;
        _logger = logger;
    }

    public async Task<ImportResult> ImportFileAsync(
        string sourcePath,
        string destinationPath,
        FileStrategy? preferredStrategy = null,
        bool verifyChecksum = true)
    {
        try
        {
            // Validate inputs
            if (!File.Exists(sourcePath))
            {
                return ImportResult.Failure($"Source file does not exist: {sourcePath}");
            }

            // Select optimal strategy
            var strategy = _strategySelector.SelectStrategy(sourcePath, destinationPath, preferredStrategy);

            _logger.LogInformation(
                "Importing file with {Strategy} strategy: {Source} → {Dest}",
                strategy,
                sourcePath,
                destinationPath);

            // Convert strategy to transfer mode
            var transferMode = ConvertToTransferMode(strategy);

            // Execute file transfer
            var actualMode = _diskTransferService.TransferFile(
                sourcePath,
                destinationPath,
                transferMode,
                overwrite: false);

            _logger.LogDebug("Transfer completed using {ActualMode} mode", actualMode);

            // Verify file integrity
            var verificationPassed = await _verificationService.VerifyFileIntegrityAsync(
                sourcePath,
                destinationPath,
                verifyChecksum);

            if (!verificationPassed)
            {
                _logger.LogError("File verification failed, rolling back import");

                // Rollback - delete destination file
                if (!_recycleBinProvider.DeleteFile(destinationPath))
                {
                    _logger.LogWarning("Failed to move destination file to recycle bin, attempting permanent delete");
                    File.Delete(destinationPath);
                }

                return ImportResult.Failure("File verification failed after transfer");
            }

            _logger.LogInformation(
                "File import successful: {Dest} ({Size} bytes)",
                destinationPath,
                new FileInfo(destinationPath).Length);

            return ImportResult.Success(destinationPath, strategy, actualMode);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "File import failed: {Source} → {Dest}", sourcePath, destinationPath);

            // Attempt cleanup if destination file was partially created
            if (File.Exists(destinationPath))
            {
                try
                {
                    _recycleBinProvider.DeleteFile(destinationPath);
                }
                catch (Exception cleanupEx)
                {
                    _logger.LogWarning(cleanupEx, "Failed to cleanup destination file after error");
                }
            }

            return ImportResult.Failure($"Import failed: {ex.Message}");
        }
    }

    private static TransferMode ConvertToTransferMode(FileStrategy strategy)
    {
        return strategy switch
        {
            FileStrategy.Copy => TransferMode.Copy,
            FileStrategy.Hardlink => TransferMode.HardLink | TransferMode.Copy, // Fallback to copy
            FileStrategy.Move => TransferMode.Move,
            FileStrategy.Symlink => TransferMode.Copy, // Symlink not yet supported in TransferMode
            _ => TransferMode.Copy
        };
    }
}

public class ImportResult
{
    public bool IsSuccess { get; private set; }
    public string? DestinationPath { get; private set; }
    public FileStrategy? RequestedStrategy { get; private set; }
    public TransferMode? ActualMode { get; private set; }
    public string? ErrorMessage { get; private set; }

    private ImportResult() { }

    public static ImportResult Success(
        string destinationPath,
        FileStrategy requestedStrategy,
        TransferMode actualMode)
    {
        return new ImportResult
        {
            IsSuccess = true,
            DestinationPath = destinationPath,
            RequestedStrategy = requestedStrategy,
            ActualMode = actualMode
        };
    }

    public static ImportResult Failure(string errorMessage)
    {
        return new ImportResult
        {
            IsSuccess = false,
            ErrorMessage = errorMessage
        };
    }
}

// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Security.Cryptography;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.MediaFiles.Import;

public interface IMediaFileVerificationService
{
    /// <summary>
    /// Verifies file integrity by comparing sizes and checksums.
    /// </summary>
    /// <param name="sourcePath">Original file path</param>
    /// <param name="destinationPath">Copied/moved file path</param>
    /// <param name="verifyChecksum">Whether to verify checksum (slower but thorough)</param>
    /// <returns>True if files match, false otherwise</returns>
    Task<bool> VerifyFileIntegrityAsync(string sourcePath, string destinationPath, bool verifyChecksum = true);

    /// <summary>
    /// Calculates MD5 checksum of a file.
    /// </summary>
    string CalculateChecksum(string filePath);
}

public class MediaFileVerificationService : IMediaFileVerificationService
{
    private readonly ILogger<MediaFileVerificationService> _logger;

    public MediaFileVerificationService(ILogger<MediaFileVerificationService> logger)
    {
        _logger = logger;
    }

    public async Task<bool> VerifyFileIntegrityAsync(
        string sourcePath,
        string destinationPath,
        bool verifyChecksum = true)
    {
        try
        {
            // Check both files exist
            if (!File.Exists(sourcePath))
            {
                _logger.LogError("Source file does not exist: {Path}", sourcePath);
                return false;
            }

            if (!File.Exists(destinationPath))
            {
                _logger.LogError("Destination file does not exist: {Path}", destinationPath);
                return false;
            }

            // Quick size check first
            var sourceInfo = new FileInfo(sourcePath);
            var destInfo = new FileInfo(destinationPath);

            if (sourceInfo.Length != destInfo.Length)
            {
                _logger.LogWarning(
                    "File size mismatch: {Source} ({SourceSize} bytes) vs {Dest} ({DestSize} bytes)",
                    sourcePath,
                    sourceInfo.Length,
                    destinationPath,
                    destInfo.Length);

                return false;
            }

            // Optional checksum verification (slower but thorough)
            if (verifyChecksum)
            {
                _logger.LogDebug("Calculating checksums for verification...");

                var sourceChecksum = await Task.Run(() => CalculateChecksum(sourcePath));
                var destChecksum = await Task.Run(() => CalculateChecksum(destinationPath));

                if (sourceChecksum != destChecksum)
                {
                    _logger.LogError(
                        "Checksum mismatch: {Source} ({SourceChecksum}) vs {Dest} ({DestChecksum})",
                        sourcePath,
                        sourceChecksum,
                        destinationPath,
                        destChecksum);

                    return false;
                }

                _logger.LogDebug("Checksum verification passed: {Checksum}", sourceChecksum);
            }

            _logger.LogDebug(
                "File integrity verified: {Source} → {Dest} ({Size} bytes)",
                sourcePath,
                destinationPath,
                sourceInfo.Length);

            return true;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to verify file integrity");
            return false;
        }
    }

    public string CalculateChecksum(string filePath)
    {
        try
        {
            // MD5 is used for file integrity verification, not cryptographic security.
            // This is an acceptable use case as we're detecting file corruption, not protecting against malicious tampering.
#pragma warning disable CA5351 // Do Not Use Broken Cryptographic Algorithms
            using var md5 = MD5.Create();
#pragma warning restore CA5351
            using var stream = File.OpenRead(filePath);

            var hashBytes = md5.ComputeHash(stream);
            return BitConverter.ToString(hashBytes).Replace("-", "").ToLowerInvariant();
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to calculate checksum for {Path}", filePath);
            throw;
        }
    }
}

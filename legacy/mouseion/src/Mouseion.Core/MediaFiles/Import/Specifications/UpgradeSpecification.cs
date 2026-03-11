// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Common.Extensions;
using Mouseion.Core.Music;
using Mouseion.Core.Qualities;

namespace Mouseion.Core.MediaFiles.Import.Specifications;

public class UpgradeSpecification : IImportSpecification
{
    private readonly IMusicFileRepository _musicFileRepository;
    private readonly ILogger<UpgradeSpecification> _logger;

    public UpgradeSpecification(
        IMusicFileRepository musicFileRepository,
        ILogger<UpgradeSpecification> logger)
    {
        _musicFileRepository = musicFileRepository;
        _logger = logger;
    }

    public async Task<ImportRejection?> IsSatisfiedByAsync(MusicFileInfo musicFileInfo, CancellationToken ct = default)
    {
        // Check if a file with the same path already exists
        var existingFile = await _musicFileRepository.FindByPathAsync(musicFileInfo.Path, ct).ConfigureAwait(false);
        return CheckUpgrade(musicFileInfo, existingFile);
    }

    public ImportRejection? IsSatisfiedBy(MusicFileInfo musicFileInfo)
    {
        var existingFile = _musicFileRepository.FindByPath(musicFileInfo.Path);
        return CheckUpgrade(musicFileInfo, existingFile);
    }

    private ImportRejection? CheckUpgrade(MusicFileInfo newFile, MusicFile? existingFile)
    {
        // No existing file = always accept (it's a new file, not an upgrade scenario)
        if (existingFile == null)
        {
            _logger.LogDebug("No existing file at path, accepting: {Path}", newFile.Path.SanitizeForLog());
            return null;
        }

        // MusicFile.Quality is QualityModel?, MusicFileInfo.Quality is Quality
        var existingQuality = existingFile.Quality;
        var newQuality = new QualityModel(newFile.Quality);

        // Check if new file is an upgrade
        if (!QualityComparer.IsUpgrade(existingQuality, newQuality))
        {
            _logger.LogDebug(
                "File is not an upgrade. Existing: {ExistingQuality}, New: {NewQuality}, Path: {Path}",
                existingQuality?.Quality.Name ?? "Unknown",
                newFile.Quality.Name,
                newFile.Path.SanitizeForLog());

            return new ImportRejection(
                ImportRejectionReason.NotQualityUpgrade,
                $"File is not an upgrade over existing quality ({existingQuality?.Quality.Name ?? "Unknown"} -> {newFile.Quality.Name})");
        }

        _logger.LogDebug(
            "File is an upgrade. Existing: {ExistingQuality}, New: {NewQuality}, Path: {Path}",
            existingQuality?.Quality.Name ?? "Unknown",
            newFile.Quality.Name,
            newFile.Path.SanitizeForLog());

        return null;
    }
}

public static class QualityUpgradeService
{
    /// <summary>
    /// Check if candidate quality is an upgrade over current quality.
    /// </summary>
    public static bool IsUpgrade(QualityModel? current, QualityModel candidate)
    {
        return QualityComparer.IsUpgrade(current, candidate);
    }

    /// <summary>
    /// Check if candidate quality is an upgrade over current, respecting cutoff.
    /// Returns false if current quality already meets cutoff (no upgrade needed).
    /// </summary>
    public static bool IsUpgradeWithCutoff(QualityModel? current, QualityModel candidate, QualityModel cutoff)
    {
        // If no current quality, always upgrade
        if (current == null)
        {
            return true;
        }

        // If current already meets cutoff, no upgrade needed
        if (QualityComparer.HasReachedCutoff(current, cutoff))
        {
            return false;
        }

        // Otherwise, check if candidate is better
        return QualityComparer.IsUpgrade(current, candidate);
    }

    /// <summary>
    /// Compare two qualities and return the better one.
    /// </summary>
    public static QualityModel GetBetterQuality(QualityModel a, QualityModel b)
    {
        return QualityComparer.Compare(a, b) >= 0 ? a : b;
    }
}

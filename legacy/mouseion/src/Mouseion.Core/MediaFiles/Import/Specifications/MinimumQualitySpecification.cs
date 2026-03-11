// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.Qualities;

namespace Mouseion.Core.MediaFiles.Import.Specifications;

public class MinimumQualitySpecification : IImportSpecification
{
    private readonly ILogger<MinimumQualitySpecification> _logger;

    public MinimumQualitySpecification(ILogger<MinimumQualitySpecification> logger)
    {
        _logger = logger;
    }

    public Task<ImportRejection?> IsSatisfiedByAsync(MusicFileInfo musicFileInfo, CancellationToken ct = default)
    {
        return Task.FromResult(IsSatisfiedBy(musicFileInfo));
    }

    public ImportRejection? IsSatisfiedBy(MusicFileInfo musicFileInfo)
    {
        if (musicFileInfo.Quality == Quality.MusicUnknown)
        {
            _logger.LogDebug("File has unknown quality: {Path}", musicFileInfo.Path);
            return new ImportRejection(
                ImportRejectionReason.MinimumQuality,
                $"File has unknown quality: {musicFileInfo.Path}");
        }

        return null;
    }
}

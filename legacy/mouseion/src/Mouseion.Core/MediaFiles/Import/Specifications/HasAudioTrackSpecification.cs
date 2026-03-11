// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;

namespace Mouseion.Core.MediaFiles.Import.Specifications;

public class HasAudioTrackSpecification : IImportSpecification
{
    private readonly ILogger<HasAudioTrackSpecification> _logger;

    public HasAudioTrackSpecification(ILogger<HasAudioTrackSpecification> logger)
    {
        _logger = logger;
    }

    public Task<ImportRejection?> IsSatisfiedByAsync(MusicFileInfo musicFileInfo, CancellationToken ct = default)
    {
        return Task.FromResult(IsSatisfiedBy(musicFileInfo));
    }

    public ImportRejection? IsSatisfiedBy(MusicFileInfo musicFileInfo)
    {
        if (musicFileInfo.Channels == 0 || musicFileInfo.SampleRate == 0)
        {
            _logger.LogDebug("File has no valid audio track: {Path}", musicFileInfo.Path);
            return new ImportRejection(
                ImportRejectionReason.NoAudioTrack,
                $"File has no valid audio track: {musicFileInfo.Path}");
        }

        if (musicFileInfo.DurationSeconds == 0)
        {
            _logger.LogDebug("File has zero duration: {Path}", musicFileInfo.Path);
            return new ImportRejection(
                ImportRejectionReason.NoAudioTrack,
                $"File has zero duration: {musicFileInfo.Path}");
        }

        return null;
    }
}

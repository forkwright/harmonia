// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.Music;

namespace Mouseion.Core.MediaFiles.Import.Aggregation;

public interface IAggregationService
{
    Task<MusicFileInfo> AugmentAsync(MusicFileInfo musicFileInfo, CancellationToken ct = default);
    MusicFileInfo Augment(MusicFileInfo musicFileInfo);
}

public class AggregationService : IAggregationService
{
    private readonly ILogger<AggregationService> _logger;

    public AggregationService(ILogger<AggregationService> logger)
    {
        _logger = logger;
    }

    public async Task<MusicFileInfo> AugmentAsync(MusicFileInfo musicFileInfo, CancellationToken ct = default)
    {
        _logger.LogDebug("Augmenting music file info: {Path}", musicFileInfo.Path);
        return await Task.FromResult(musicFileInfo);
    }

    public MusicFileInfo Augment(MusicFileInfo musicFileInfo)
    {
        _logger.LogDebug("Augmenting music file info: {Path}", musicFileInfo.Path);
        return musicFileInfo;
    }
}

// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
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

    public Task<ImportRejection?> IsSatisfiedByAsync(MusicFileInfo musicFileInfo, CancellationToken ct = default)
    {
        return Task.FromResult(IsSatisfiedBy(musicFileInfo));
    }

    public ImportRejection? IsSatisfiedBy(MusicFileInfo musicFileInfo)
    {
        return null;
    }
}

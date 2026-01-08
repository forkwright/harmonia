// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.Music;
using Mouseion.Core.Qualities;

namespace Mouseion.Core.MediaFiles.Import.Specifications;

public class UpgradeSpecification : IImportSpecification
{
    public Task<ImportRejection?> IsSatisfiedByAsync(MusicFileInfo musicFileInfo, CancellationToken ct = default)
    {
        return Task.FromResult(IsSatisfiedBy(musicFileInfo));
    }

    public ImportRejection? IsSatisfiedBy(MusicFileInfo musicFileInfo)
    {
        return null;
    }
}

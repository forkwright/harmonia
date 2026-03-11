// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.MediaFiles.Import;

public interface IImportSpecification
{
    Task<ImportRejection?> IsSatisfiedByAsync(MusicFileInfo musicFileInfo, CancellationToken ct = default);
    ImportRejection? IsSatisfiedBy(MusicFileInfo musicFileInfo);
}

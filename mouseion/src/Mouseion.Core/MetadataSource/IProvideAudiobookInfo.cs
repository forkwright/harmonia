// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Audiobooks;

namespace Mouseion.Core.MetadataSource;

public interface IProvideAudiobookInfo
{
    Task<Audiobook?> GetByAsinAsync(string asin, CancellationToken ct = default);
    Task<Audiobook?> GetByIdAsync(int id, CancellationToken ct = default);
    Task<List<Audiobook>> SearchByTitleAsync(string title, CancellationToken ct = default);
    Task<List<Audiobook>> SearchByAuthorAsync(string author, CancellationToken ct = default);
    Task<List<Audiobook>> SearchByNarratorAsync(string narrator, CancellationToken ct = default);
    Task<List<Audiobook>> GetTrendingAsync(CancellationToken ct = default);
    Task<List<Audiobook>> GetPopularAsync(CancellationToken ct = default);
}

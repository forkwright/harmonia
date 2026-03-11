// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Books;

namespace Mouseion.Core.MetadataSource;

public interface IProvideBookInfo
{
    Task<Book?> GetByExternalIdAsync(string externalId, CancellationToken ct = default);
    Task<Book?> GetByIdAsync(int id, CancellationToken ct = default);
    Task<List<Book>> SearchByTitleAsync(string title, CancellationToken ct = default);
    Task<List<Book>> SearchByAuthorAsync(string author, CancellationToken ct = default);
    Task<List<Book>> SearchByIsbnAsync(string isbn, CancellationToken ct = default);
    Task<List<Book>> GetTrendingAsync(CancellationToken ct = default);
    Task<List<Book>> GetPopularAsync(CancellationToken ct = default);
}

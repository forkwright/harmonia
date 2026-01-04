// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Tags;

public interface ITagRepository : IBasicRepository<Tag>
{
    Tag? GetByLabel(string label);
    Tag? FindByLabel(string label);
    List<Tag> GetTags(HashSet<int> tagIds);

    Task<Tag?> GetByLabelAsync(string label, CancellationToken ct = default);
    Task<Tag?> FindByLabelAsync(string label, CancellationToken ct = default);
    Task<List<Tag>> GetTagsAsync(HashSet<int> tagIds, CancellationToken ct = default);
}

// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Tags;

public interface ITagService
{
    Tag GetTag(int tagId);
    Tag GetTag(string tag);
    List<Tag> GetTags(IEnumerable<int> ids);
    List<Tag> All();
    Tag Add(Tag tag);
    Tag Update(Tag tag);
    void Delete(int tagId);

    Task<Tag> GetTagAsync(int tagId, CancellationToken ct = default);
    Task<Tag> GetTagAsync(string tag, CancellationToken ct = default);
    Task<List<Tag>> GetTagsAsync(IEnumerable<int> ids, CancellationToken ct = default);
    Task<List<Tag>> AllAsync(CancellationToken ct = default);
    Task<Tag> AddAsync(Tag tag, CancellationToken ct = default);
    Task<Tag> UpdateAsync(Tag tag, CancellationToken ct = default);
    Task DeleteAsync(int tagId, CancellationToken ct = default);
}

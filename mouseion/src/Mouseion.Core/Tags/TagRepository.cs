// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Tags;

public class TagRepository : BasicRepository<Tag>, ITagRepository
{
    public TagRepository(IDatabase database)
        : base(database, "Tags")
    {
    }

    public Tag? GetByLabel(string label)
    {
        var model = FindByLabel(label);

        if (model == null)
        {
            throw new InvalidOperationException($"Tag with label '{label}' not found");
        }

        return model;
    }

    public Tag? FindByLabel(string label)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Tag>(
            "SELECT * FROM \"Tags\" WHERE \"Label\" = @Label",
            new { Label = label });
    }

    public List<Tag> GetTags(HashSet<int> tagIds)
    {
        if (tagIds == null || tagIds.Count == 0)
        {
            return new List<Tag>();
        }

        using var conn = _database.OpenConnection();
        return conn.Query<Tag>(
            "SELECT * FROM \"Tags\" WHERE \"Id\" = ANY(@Ids)",
            new { Ids = tagIds.ToArray() }).ToList();
    }

    public async Task<Tag?> GetByLabelAsync(string label, CancellationToken ct = default)
    {
        var model = await FindByLabelAsync(label, ct).ConfigureAwait(false);

        if (model == null)
        {
            throw new InvalidOperationException($"Tag with label '{label}' not found");
        }

        return model;
    }

    public async Task<Tag?> FindByLabelAsync(string label, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Tag>(
            "SELECT * FROM \"Tags\" WHERE \"Label\" = @Label",
            new { Label = label }).ConfigureAwait(false);
    }

    public async Task<List<Tag>> GetTagsAsync(HashSet<int> tagIds, CancellationToken ct = default)
    {
        if (tagIds == null || tagIds.Count == 0)
        {
            return new List<Tag>();
        }

        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Tag>(
            "SELECT * FROM \"Tags\" WHERE \"Id\" = ANY(@Ids)",
            new { Ids = tagIds.ToArray() }).ConfigureAwait(false);
        return result.ToList();
    }
}

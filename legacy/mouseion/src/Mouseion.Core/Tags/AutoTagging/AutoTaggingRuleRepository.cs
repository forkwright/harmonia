// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Tags.AutoTagging;

public class AutoTaggingRuleRepository : BasicRepository<AutoTaggingRule>, IAutoTaggingRuleRepository
{
    public AutoTaggingRuleRepository(IDatabase database)
        : base(database, "AutoTaggingRules")
    {
    }

    public async Task<List<AutoTaggingRule>> GetEnabledRulesAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<AutoTaggingRule>(
            "SELECT * FROM \"AutoTaggingRules\" WHERE \"Enabled\" = true ORDER BY \"Name\"")
            .ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<List<AutoTaggingRule>> GetEnabledRulesForMediaTypeAsync(MediaType mediaType, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<AutoTaggingRule>(
            "SELECT * FROM \"AutoTaggingRules\" WHERE \"Enabled\" = true AND (\"MediaTypeFilter\" IS NULL OR \"MediaTypeFilter\" = @MediaType) ORDER BY \"Name\"",
            new { MediaType = (int)mediaType })
            .ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<List<AutoTaggingRule>> GetRulesByTagIdAsync(int tagId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<AutoTaggingRule>(
            "SELECT * FROM \"AutoTaggingRules\" WHERE \"TagId\" = @TagId ORDER BY \"Name\"",
            new { TagId = tagId })
            .ConfigureAwait(false);
        return result.ToList();
    }

    public List<AutoTaggingRule> GetEnabledRules()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<AutoTaggingRule>(
            "SELECT * FROM \"AutoTaggingRules\" WHERE \"Enabled\" = true ORDER BY \"Name\"")
            .ToList();
    }

    public List<AutoTaggingRule> GetEnabledRulesForMediaType(MediaType mediaType)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<AutoTaggingRule>(
            "SELECT * FROM \"AutoTaggingRules\" WHERE \"Enabled\" = true AND (\"MediaTypeFilter\" IS NULL OR \"MediaTypeFilter\" = @MediaType) ORDER BY \"Name\"",
            new { MediaType = (int)mediaType })
            .ToList();
    }

    public List<AutoTaggingRule> GetRulesByTagId(int tagId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<AutoTaggingRule>(
            "SELECT * FROM \"AutoTaggingRules\" WHERE \"TagId\" = @TagId ORDER BY \"Name\"",
            new { TagId = tagId })
            .ToList();
    }
}

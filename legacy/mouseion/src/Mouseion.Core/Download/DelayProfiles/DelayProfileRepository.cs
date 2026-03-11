// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Download.DelayProfiles;

public interface IDelayProfileRepository : IBasicRepository<DelayProfile>
{
    Task<IEnumerable<DelayProfile>> GetEnabledAsync(CancellationToken ct = default);
    Task<DelayProfile?> GetBestMatchAsync(MediaType mediaType, IEnumerable<int> tagIds, CancellationToken ct = default);
}

public class DelayProfileRepository : BasicRepository<DelayProfile>, IDelayProfileRepository
{
    public DelayProfileRepository(IDatabase database) : base(database) { }

    public async Task<IEnumerable<DelayProfile>> GetEnabledAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryAsync<DelayProfile>(
            @"SELECT * FROM ""DelayProfiles"" WHERE ""Enabled"" = 1 ORDER BY ""Order"" ASC"
        ).ConfigureAwait(false);
    }

    public async Task<DelayProfile?> GetBestMatchAsync(MediaType mediaType, IEnumerable<int> tagIds, CancellationToken ct = default)
    {
        var profiles = await GetEnabledAsync(ct).ConfigureAwait(false);
        var tagSet = new HashSet<int>(tagIds);

        // Find best match: media type match + tag overlap, ordered by priority
        return profiles
            .Where(p => !p.MediaType.HasValue || p.MediaType.Value == mediaType)
            .Where(p =>
            {
                if (string.IsNullOrEmpty(p.Tags)) return true; // No tags = matches everything
                var profileTags = p.Tags.Split(',', StringSplitOptions.RemoveEmptyEntries)
                    .Select(t => int.TryParse(t.Trim(), out var id) ? id : -1)
                    .Where(id => id >= 0);
                return profileTags.Any(tagSet.Contains);
            })
            .OrderBy(p => p.Order)
            .FirstOrDefault();
    }
}

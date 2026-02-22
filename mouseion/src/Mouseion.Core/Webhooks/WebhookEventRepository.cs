// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Webhooks;

public class WebhookEventRepository : BasicRepository<WebhookEvent>
{
    public WebhookEventRepository(IDatabase db) : base(db) { }

    public async Task<WebhookEvent?> FindRecentDuplicate(WebhookSource source, string externalItemId, string eventType, TimeSpan window, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<WebhookEvent>(
            @"SELECT * FROM ""WebhookEvents""
              WHERE ""Source"" = @Source
                AND ""ExternalItemId"" = @ExternalItemId
                AND ""EventType"" = @EventType
                AND ""ReceivedAt"" > @CutoffTime
              ORDER BY ""ReceivedAt"" DESC LIMIT 1",
            new
            {
                Source = (int)source,
                ExternalItemId = externalItemId,
                EventType = eventType,
                CutoffTime = DateTime.UtcNow - window
            }).ConfigureAwait(false);
    }

    public async Task<List<WebhookEvent>> GetUnprocessed(int limit = 100, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<WebhookEvent>(
            @"SELECT * FROM ""WebhookEvents""
              WHERE ""Processed"" = 0
              ORDER BY ""ReceivedAt"" ASC
              LIMIT @Limit",
            new { Limit = limit }).ConfigureAwait(false);
        return result.ToList();
    }
}

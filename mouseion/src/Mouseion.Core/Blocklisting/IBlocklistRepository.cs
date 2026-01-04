// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Blocklisting;

public interface IBlocklistRepository : IBasicRepository<Blocklist>
{
    Task<List<Blocklist>> GetByMediaItemIdAsync(int mediaItemId, CancellationToken ct = default);
    Task<List<Blocklist>> GetBySourceTitleAsync(int mediaItemId, string sourceTitle, CancellationToken ct = default);
    Task<List<Blocklist>> GetByTorrentInfoHashAsync(int mediaItemId, string torrentInfoHash, CancellationToken ct = default);
    Task DeleteByMediaItemIdsAsync(List<int> mediaItemIds, CancellationToken ct = default);

    List<Blocklist> GetByMediaItemId(int mediaItemId);
    List<Blocklist> GetBySourceTitle(int mediaItemId, string sourceTitle);
    List<Blocklist> GetByTorrentInfoHash(int mediaItemId, string torrentInfoHash);
    void DeleteByMediaItemIds(List<int> mediaItemIds);
}

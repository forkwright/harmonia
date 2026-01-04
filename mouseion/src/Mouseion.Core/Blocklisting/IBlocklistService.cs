// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Blocklisting;

public interface IBlocklistService
{
    Task<bool> IsBlocklistedAsync(int mediaItemId, string sourceTitle, DownloadProtocol protocol, string? torrentInfoHash = null, CancellationToken ct = default);
    Task<List<Blocklist>> GetAllAsync(CancellationToken ct = default);
    Task<List<Blocklist>> GetByMediaItemIdAsync(int mediaItemId, CancellationToken ct = default);
    Task AddAsync(Blocklist blocklist, CancellationToken ct = default);
    Task DeleteAsync(int id, CancellationToken ct = default);
    Task DeleteManyAsync(List<int> ids, CancellationToken ct = default);
    Task ClearAllAsync(CancellationToken ct = default);

    bool IsBlocklisted(int mediaItemId, string sourceTitle, DownloadProtocol protocol, string? torrentInfoHash = null);
    List<Blocklist> GetAll();
    List<Blocklist> GetByMediaItemId(int mediaItemId);
    void Add(Blocklist blocklist);
    void Delete(int id);
    void DeleteMany(List<int> ids);
    void ClearAll();
}

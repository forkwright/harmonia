// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Blocklisting;

public class BlocklistService : IBlocklistService
{
    private readonly IBlocklistRepository _repository;

    public BlocklistService(IBlocklistRepository repository)
    {
        _repository = repository;
    }

    public async Task<bool> IsBlocklistedAsync(int mediaItemId, string sourceTitle, DownloadProtocol protocol, string? torrentInfoHash = null, CancellationToken ct = default)
    {
        if (protocol == DownloadProtocol.Torrent && !string.IsNullOrWhiteSpace(torrentInfoHash))
        {
            var hashMatches = await _repository.GetByTorrentInfoHashAsync(mediaItemId, torrentInfoHash, ct).ConfigureAwait(false);
            if (hashMatches.Any(b => b.TorrentInfoHash != null &&
                                     b.TorrentInfoHash.Equals(torrentInfoHash, StringComparison.OrdinalIgnoreCase)))
            {
                return true;
            }
        }

        var titleMatches = await _repository.GetBySourceTitleAsync(mediaItemId, sourceTitle, ct).ConfigureAwait(false);
        return titleMatches.Any(b => b.Protocol == protocol);
    }

    public bool IsBlocklisted(int mediaItemId, string sourceTitle, DownloadProtocol protocol, string? torrentInfoHash = null)
    {
        if (protocol == DownloadProtocol.Torrent && !string.IsNullOrWhiteSpace(torrentInfoHash))
        {
            var hashMatches = _repository.GetByTorrentInfoHash(mediaItemId, torrentInfoHash);
            if (hashMatches.Any(b => b.TorrentInfoHash != null &&
                                     b.TorrentInfoHash.Equals(torrentInfoHash, StringComparison.OrdinalIgnoreCase)))
            {
                return true;
            }
        }

        var titleMatches = _repository.GetBySourceTitle(mediaItemId, sourceTitle);
        return titleMatches.Any(b => b.Protocol == protocol);
    }

    public async Task<List<Blocklist>> GetAllAsync(CancellationToken ct = default)
    {
        var all = await _repository.AllAsync(ct).ConfigureAwait(false);
        return all.OrderByDescending(b => b.Date).ToList();
    }

    public List<Blocklist> GetAll()
    {
        var all = _repository.All();
        return all.OrderByDescending(b => b.Date).ToList();
    }

    public async Task<List<Blocklist>> GetByMediaItemIdAsync(int mediaItemId, CancellationToken ct = default)
    {
        return await _repository.GetByMediaItemIdAsync(mediaItemId, ct).ConfigureAwait(false);
    }

    public List<Blocklist> GetByMediaItemId(int mediaItemId)
    {
        return _repository.GetByMediaItemId(mediaItemId);
    }

    public async Task AddAsync(Blocklist blocklist, CancellationToken ct = default)
    {
        blocklist.Date = DateTime.UtcNow;
        await _repository.InsertAsync(blocklist, ct).ConfigureAwait(false);
    }

    public void Add(Blocklist blocklist)
    {
        blocklist.Date = DateTime.UtcNow;
        _repository.Insert(blocklist);
    }

    public async Task DeleteAsync(int id, CancellationToken ct = default)
    {
        await _repository.DeleteAsync(id, ct).ConfigureAwait(false);
    }

    public void Delete(int id)
    {
        _repository.Delete(id);
    }

    public async Task DeleteManyAsync(List<int> ids, CancellationToken ct = default)
    {
        foreach (var id in ids)
        {
            await _repository.DeleteAsync(id, ct).ConfigureAwait(false);
        }
    }

    public void DeleteMany(List<int> ids)
    {
        foreach (var id in ids)
        {
            _repository.Delete(id);
        }
    }

    public async Task ClearAllAsync(CancellationToken ct = default)
    {
        var all = await _repository.AllAsync(ct).ConfigureAwait(false);
        foreach (var item in all)
        {
            await _repository.DeleteAsync(item.Id, ct).ConfigureAwait(false);
        }
    }

    public void ClearAll()
    {
        var all = _repository.All();
        foreach (var item in all)
        {
            _repository.Delete(item.Id);
        }
    }
}

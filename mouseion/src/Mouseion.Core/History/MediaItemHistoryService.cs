// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.History;

public class MediaItemHistoryService : IMediaItemHistoryService
{
    private readonly IMediaItemHistoryRepository _repository;

    public MediaItemHistoryService(IMediaItemHistoryRepository repository)
    {
        _repository = repository;
    }

    public async Task<List<MediaItemHistory>> GetAllAsync(CancellationToken ct = default)
    {
        var all = await _repository.AllAsync(ct).ConfigureAwait(false);
        return all.OrderByDescending(h => h.Date).ToList();
    }

    public List<MediaItemHistory> GetAll()
    {
        var all = _repository.All();
        return all.OrderByDescending(h => h.Date).ToList();
    }

    public async Task<List<MediaItemHistory>> GetByMediaItemIdAsync(int mediaItemId, CancellationToken ct = default)
    {
        return await _repository.GetByMediaItemIdAsync(mediaItemId, ct).ConfigureAwait(false);
    }

    public List<MediaItemHistory> GetByMediaItemId(int mediaItemId)
    {
        return _repository.GetByMediaItemId(mediaItemId);
    }

    public async Task<List<MediaItemHistory>> FindByDownloadIdAsync(string downloadId, CancellationToken ct = default)
    {
        return await _repository.FindByDownloadIdAsync(downloadId, ct).ConfigureAwait(false);
    }

    public List<MediaItemHistory> FindByDownloadId(string downloadId)
    {
        return _repository.FindByDownloadId(downloadId);
    }

    public async Task<MediaItemHistory?> MostRecentForMediaItemAsync(int mediaItemId, HistoryEventType eventType, CancellationToken ct = default)
    {
        return await _repository.MostRecentForMediaItemAsync(mediaItemId, eventType, ct).ConfigureAwait(false);
    }

    public MediaItemHistory? MostRecentForMediaItem(int mediaItemId, HistoryEventType eventType)
    {
        return _repository.MostRecentForMediaItem(mediaItemId, eventType);
    }

    public async Task<List<MediaItemHistory>> SinceAsync(DateTime date, CancellationToken ct = default)
    {
        return await _repository.SinceAsync(date, ct).ConfigureAwait(false);
    }

    public List<MediaItemHistory> Since(DateTime date)
    {
        return _repository.Since(date);
    }

    public async Task<(List<MediaItemHistory> records, int totalRecords)> GetPagedAsync(int page, int pageSize, string? sortKey, string? sortDir, CancellationToken ct = default)
    {
        var records = await _repository.GetPagedAsync(page, pageSize, sortKey, sortDir, ct).ConfigureAwait(false);
        var totalRecords = await _repository.CountAsync(ct).ConfigureAwait(false);
        return (records, totalRecords);
    }

    public async Task AddAsync(MediaItemHistory history, CancellationToken ct = default)
    {
        history.Date = DateTime.UtcNow;
        await _repository.InsertAsync(history, ct).ConfigureAwait(false);
    }

    public void Add(MediaItemHistory history)
    {
        history.Date = DateTime.UtcNow;
        _repository.Insert(history);
    }

    public async Task DeleteAsync(int id, CancellationToken ct = default)
    {
        await _repository.DeleteAsync(id, ct).ConfigureAwait(false);
    }

    public void Delete(int id)
    {
        _repository.Delete(id);
    }

    public async Task PurgeOlderThanAsync(DateTime date, CancellationToken ct = default)
    {
        var oldRecords = await _repository.AllAsync(ct).ConfigureAwait(false);
        var toDelete = oldRecords.Where(h => h.Date < date).ToList();

        foreach (var record in toDelete)
        {
            await _repository.DeleteAsync(record.Id, ct).ConfigureAwait(false);
        }
    }

    public void PurgeOlderThan(DateTime date)
    {
        var oldRecords = _repository.All();
        var toDelete = oldRecords.Where(h => h.Date < date).ToList();

        foreach (var record in toDelete)
        {
            _repository.Delete(record.Id);
        }
    }

    public static Task HandleGrabbedAsync(int mediaItemId, string sourceTitle, string downloadId, CancellationToken ct = default)
    {
        return Task.CompletedTask;
    }

    public static Task HandleImportedAsync(int mediaItemId, string sourceTitle, CancellationToken ct = default)
    {
        return Task.CompletedTask;
    }

    public static Task HandleFailedAsync(int mediaItemId, string sourceTitle, string downloadId, string message, CancellationToken ct = default)
    {
        return Task.CompletedTask;
    }

    public static Task HandleDeletedAsync(int mediaItemId, string filePath, CancellationToken ct = default)
    {
        return Task.CompletedTask;
    }

    public static Task HandleRenamedAsync(int mediaItemId, string oldPath, string newPath, CancellationToken ct = default)
    {
        return Task.CompletedTask;
    }

    public static Task HandleIgnoredAsync(int mediaItemId, string sourceTitle, string downloadId, string reason, CancellationToken ct = default)
    {
        return Task.CompletedTask;
    }
}

// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.History;

public interface IMediaItemHistoryService
{
    Task<List<MediaItemHistory>> GetAllAsync(CancellationToken ct = default);
    Task<List<MediaItemHistory>> GetByMediaItemIdAsync(int mediaItemId, CancellationToken ct = default);
    Task<List<MediaItemHistory>> FindByDownloadIdAsync(string downloadId, CancellationToken ct = default);
    Task<MediaItemHistory?> MostRecentForMediaItemAsync(int mediaItemId, HistoryEventType eventType, CancellationToken ct = default);
    Task<List<MediaItemHistory>> SinceAsync(DateTime date, CancellationToken ct = default);
    Task<(List<MediaItemHistory> records, int totalRecords)> GetPagedAsync(int page, int pageSize, string? sortKey, string? sortDir, CancellationToken ct = default);
    Task AddAsync(MediaItemHistory history, CancellationToken ct = default);
    Task DeleteAsync(int id, CancellationToken ct = default);
    Task PurgeOlderThanAsync(DateTime date, CancellationToken ct = default);

    List<MediaItemHistory> GetAll();
    List<MediaItemHistory> GetByMediaItemId(int mediaItemId);
    List<MediaItemHistory> FindByDownloadId(string downloadId);
    MediaItemHistory? MostRecentForMediaItem(int mediaItemId, HistoryEventType eventType);
    List<MediaItemHistory> Since(DateTime date);
    void Add(MediaItemHistory history);
    void Delete(int id);
    void PurgeOlderThan(DateTime date);
}

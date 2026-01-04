// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.History;

public interface IMediaItemHistoryRepository : IBasicRepository<MediaItemHistory>
{
    Task<List<MediaItemHistory>> GetByMediaItemIdAsync(int mediaItemId, CancellationToken ct = default);
    Task<List<MediaItemHistory>> FindByDownloadIdAsync(string downloadId, CancellationToken ct = default);
    Task<MediaItemHistory?> MostRecentForMediaItemAsync(int mediaItemId, HistoryEventType eventType, CancellationToken ct = default);
    Task<List<MediaItemHistory>> SinceAsync(DateTime date, CancellationToken ct = default);
    Task<List<MediaItemHistory>> GetPagedAsync(int page, int pageSize, string? sortKey, string? sortDir, CancellationToken ct = default);

    List<MediaItemHistory> GetByMediaItemId(int mediaItemId);
    List<MediaItemHistory> FindByDownloadId(string downloadId);
    MediaItemHistory? MostRecentForMediaItem(int mediaItemId, HistoryEventType eventType);
    List<MediaItemHistory> Since(DateTime date);
}

// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.ImportLists.ImportExclusions;

namespace Mouseion.Core.ImportLists;

public interface IImportListSyncService
{
    Task<ImportListFetchResult> SyncAllAsync(CancellationToken cancellationToken = default);
    Task<ImportListFetchResult> SyncListAsync(int listId, CancellationToken cancellationToken = default);
}

public class ImportListSyncService : IImportListSyncService
{
    private readonly IImportListFactory _factory;
    private readonly IImportListExclusionService _exclusionService;
    private readonly ILogger<ImportListSyncService> _logger;

    public ImportListSyncService(IImportListFactory factory, IImportListExclusionService exclusionService, ILogger<ImportListSyncService> logger)
    {
        _factory = factory;
        _exclusionService = exclusionService;
        _logger = logger;
    }

    public async Task<ImportListFetchResult> SyncAllAsync(CancellationToken cancellationToken = default)
    {
        var enabledLists = _factory.GetEnabled();
        if (!enabledLists.Any())
        {
            _logger.LogDebug("No enabled import lists, skipping sync");
            return new ImportListFetchResult();
        }

        var result = new ImportListFetchResult();
        var exclusions = _exclusionService.GetAll();

        foreach (var list in enabledLists)
        {
            try
            {
                _logger.LogInformation("Syncing import list: {Name}", list.Name);
                var fetchResult = await list.FetchAsync(cancellationToken);
                result.SyncedLists++;
                result.AnyFailure |= fetchResult.AnyFailure;
                var filteredItems = FilterExcludedItems(fetchResult.Items, exclusions);
                result.Items.AddRange(filteredItems);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to sync import list: {Name}", list.Name);
                result.AnyFailure = true;
            }
        }

        _logger.LogInformation("Import list sync complete. Fetched {Count} items from {Lists} lists", result.Items.Count, result.SyncedLists);
        return result;
    }

    public async Task<ImportListFetchResult> SyncListAsync(int listId, CancellationToken cancellationToken = default)
    {
        var list = _factory.Get(listId);
        _logger.LogInformation("Syncing single import list: {Name}", list.Name);
        var result = await list.FetchAsync(cancellationToken);
        var exclusions = _exclusionService.GetAll();
        result.Items = FilterExcludedItems(result.Items, exclusions);
        result.SyncedLists = 1;
        return result;
    }

    private List<ImportListItem> FilterExcludedItems(List<ImportListItem> items, List<ImportListExclusion> exclusions)
    {
        return items.Where(item =>
        {
            var isExcluded = exclusions.Any(ex =>
                (ex.MediaType == item.MediaType) &&
                ((ex.TmdbId > 0 && ex.TmdbId == item.TmdbId) ||
                 (ex.ImdbId != null && ex.ImdbId == item.ImdbId) ||
                 (ex.TvdbId > 0 && ex.TvdbId == item.TvdbId) ||
                 (ex.GoodreadsId > 0 && ex.GoodreadsId == item.GoodreadsId) ||
                 (ex.Isbn != null && ex.Isbn == item.Isbn) ||
                 (ex.MusicBrainzId != Guid.Empty && ex.MusicBrainzId == item.MusicBrainzId) ||
                 (ex.Asin != null && ex.Asin == item.Asin))
            );
            if (isExcluded) _logger.LogDebug("Item excluded: {Title} ({Year})", item.Title, item.Year);
            return !isExcluded;
        }).ToList();
    }
}

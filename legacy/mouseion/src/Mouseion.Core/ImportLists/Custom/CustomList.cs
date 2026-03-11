// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;

namespace Mouseion.Core.ImportLists.Custom;

public class CustomList : ImportListBase<CustomListSettings>
{
    public CustomList(ILogger<CustomList> logger) : base(logger) { }

    public override string Name => "Custom List";
    public override ImportListType ListType => ImportListType.Custom;
    public override TimeSpan MinRefreshInterval => TimeSpan.FromHours(24);
    public override bool Enabled => true;
    public override bool EnableAuto => false;

    public override Task<ImportListFetchResult> FetchAsync(CancellationToken cancellationToken = default)
    {
        Logger.LogInformation("Fetching custom list entries");
        var items = Settings.Entries.Select(entry => new ImportListItem
        {
            MediaType = Settings.MediaType,
            Title = entry.Title,
            Year = entry.Year,
            TmdbId = entry.TmdbId,
            ImdbId = entry.ImdbId
        }).ToList();

        return Task.FromResult(new ImportListFetchResult { Items = CleanupListItems(items), SyncedLists = 1 });
    }
}

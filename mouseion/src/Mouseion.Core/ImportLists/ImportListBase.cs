// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net.Http;
using System.Text.Json;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.ImportLists;

public abstract class ImportListBase<TSettings> : IImportList
    where TSettings : ImportListSettingsBase, new()
{
    protected readonly ILogger Logger;

    protected ImportListBase(ILogger logger)
    {
        Logger = logger;
    }

    public abstract string Name { get; }
    public abstract ImportListType ListType { get; }
    public abstract TimeSpan MinRefreshInterval { get; }
    public abstract bool Enabled { get; }
    public abstract bool EnableAuto { get; }
    public ImportListDefinition Definition { get; set; } = new();

    protected TSettings Settings
    {
        get
        {
            if (string.IsNullOrEmpty(Definition.Settings))
            {
                return new TSettings();
            }
            return System.Text.Json.JsonSerializer.Deserialize<TSettings>(Definition.Settings) ?? new TSettings();
        }
    }

    public abstract Task<ImportListFetchResult> FetchAsync(CancellationToken cancellationToken = default);

    public virtual async Task<bool> TestAsync(CancellationToken cancellationToken = default)
    {
        try
        {
            var result = await FetchAsync(cancellationToken);
            return !result.AnyFailure;
        }
        catch (HttpRequestException ex)
        {
            Logger.LogError(ex, "Network error testing import list {Name}", Name);
            return false;
        }
        catch (JsonException ex)
        {
            Logger.LogError(ex, "Failed to parse response testing import list {Name}", Name);
            return false;
        }
        catch (TaskCanceledException ex)
        {
            Logger.LogWarning(ex, "Request timed out or was cancelled testing import list {Name}", Name);
            return false;
        }
    }

    protected List<ImportListItem> CleanupListItems(IEnumerable<ImportListItem> items)
    {
        var result = items.ToList();
        foreach (var item in result)
        {
            item.ListId = Definition.Id;
        }
        return result;
    }
}

// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.Authors;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.MediaItems;

public abstract class AddMediaItemService<TMediaItem, TRepository>
    where TMediaItem : MediaItem, new()
    where TRepository : IBasicRepository<TMediaItem>
{
    protected readonly TRepository Repository;
    protected readonly IAuthorRepository AuthorRepository;
    protected readonly ILogger Logger;

    protected AddMediaItemService(
        TRepository repository,
        IAuthorRepository authorRepository,
        ILogger logger)
    {
        Repository = repository;
        AuthorRepository = authorRepository;
        Logger = logger;
    }

    public async Task<TMediaItem> AddItemAsync(TMediaItem item, CancellationToken ct = default)
    {
        ValidateItem(item);

        // Verify author exists
        if (item.AuthorId.HasValue)
        {
            var author = await AuthorRepository.FindAsync(item.AuthorId.Value, ct).ConfigureAwait(false);
            if (author == null)
            {
                throw new ArgumentException($"Author with ID {item.AuthorId.Value} not found", nameof(item));
            }
        }

        // Check for existing item
        if (item.AuthorId.HasValue)
        {
            var existing = await FindByTitleAsync(item.GetTitle(), item.GetYear(), ct).ConfigureAwait(false);
            if (existing != null && existing.AuthorId == item.AuthorId)
            {
                LogItemExists(existing);
                return existing;
            }
        }

        // Set defaults
        item.Added = DateTime.UtcNow;
        item.Monitored = true;

        var added = await Repository.InsertAsync(item, ct).ConfigureAwait(false);
        LogItemAdded(added);

        return added;
    }

    public TMediaItem AddItem(TMediaItem item)
    {
        ValidateItem(item);

        // Verify author exists
        if (item.AuthorId.HasValue)
        {
            var author = AuthorRepository.Find(item.AuthorId.Value);
            if (author == null)
            {
                throw new ArgumentException($"Author with ID {item.AuthorId.Value} not found", nameof(item));
            }
        }

        // Check for existing item
        if (item.AuthorId.HasValue)
        {
            var existing = FindByTitle(item.GetTitle(), item.GetYear());
            if (existing != null && existing.AuthorId == item.AuthorId)
            {
                LogItemExists(existing);
                return existing;
            }
        }

        // Set defaults
        item.Added = DateTime.UtcNow;
        item.Monitored = true;

        var added = Repository.Insert(item);
        LogItemAdded(added);

        return added;
    }

    public async Task<List<TMediaItem>> AddItemsAsync(List<TMediaItem> items, CancellationToken ct = default)
    {
        var addedItems = new List<TMediaItem>();

        foreach (var item in items)
        {
            try
            {
                var added = await AddItemAsync(item, ct).ConfigureAwait(false);
                addedItems.Add(added);
            }
            catch (Exception ex)
            {
                Logger.LogError(ex, "Error adding item: {ItemTitle}", item.GetTitle());
            }
        }

        return addedItems;
    }

    public List<TMediaItem> AddItems(List<TMediaItem> items)
    {
        var addedItems = new List<TMediaItem>();

        foreach (var item in items)
        {
            try
            {
                var added = AddItem(item);
                addedItems.Add(added);
            }
            catch (Exception ex)
            {
                Logger.LogError(ex, "Error adding item: {ItemTitle}", item.GetTitle());
            }
        }

        return addedItems;
    }

    protected void ValidateItem(TMediaItem item)
    {
        if (string.IsNullOrWhiteSpace(item.GetTitle()))
        {
            throw new ArgumentException("Title is required", nameof(item));
        }

        if (item.QualityProfileId <= 0)
        {
            throw new ArgumentException("Quality profile ID must be set", nameof(item));
        }

        // Allow derived classes to add custom validation
        CustomValidation(item);
    }

    // Abstract methods for media-specific operations
    protected abstract Task<TMediaItem?> FindByTitleAsync(string title, int year, CancellationToken ct = default);
    protected abstract TMediaItem? FindByTitle(string title, int year);
    protected abstract void LogItemAdded(TMediaItem item);
    protected abstract void LogItemExists(TMediaItem item);

    // Virtual method for custom validation (optional override)
    protected virtual void CustomValidation(TMediaItem item)
    {
        // Default: no custom validation
    }
}

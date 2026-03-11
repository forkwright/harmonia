// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Common.Extensions;

namespace Mouseion.Core.Movies;

public interface IAddCollectionService
{
    Task<Collection> AddCollectionAsync(Collection collection, CancellationToken ct = default);
    Collection AddCollection(Collection collection);
}

public class AddCollectionService : IAddCollectionService
{
    private readonly ICollectionRepository _collectionRepository;
    private readonly ILogger<AddCollectionService> _logger;

    public AddCollectionService(
        ICollectionRepository collectionRepository,
        ILogger<AddCollectionService> logger)
    {
        _collectionRepository = collectionRepository;
        _logger = logger;
    }

    public async Task<Collection> AddCollectionAsync(Collection collection, CancellationToken ct = default)
    {
        ValidateCollection(collection);

        if (!string.IsNullOrWhiteSpace(collection.TmdbId))
        {
            var existing = await _collectionRepository.FindByTmdbIdAsync(collection.TmdbId, ct).ConfigureAwait(false);
            if (existing != null)
            {
                _logger.LogInformation("Collection already exists: {CollectionTitle} - TMDB ID: {TmdbId}",
                    collection.Title.SanitizeForLog(), collection.TmdbId?.SanitizeForLog());
                return existing;
            }
        }

        collection.Added = DateTime.UtcNow;
        collection.Monitored = true;

        var added = await _collectionRepository.InsertAsync(collection, ct).ConfigureAwait(false);
        _logger.LogInformation("Added collection: {CollectionTitle} - TMDB ID: {TmdbId}",
            added.Title.SanitizeForLog(), added.TmdbId?.SanitizeForLog());

        return added;
    }

    public Collection AddCollection(Collection collection)
    {
        ValidateCollection(collection);

        if (!string.IsNullOrWhiteSpace(collection.TmdbId))
        {
            var existing = _collectionRepository.FindByTmdbId(collection.TmdbId);
            if (existing != null)
            {
                _logger.LogInformation("Collection already exists: {CollectionTitle} - TMDB ID: {TmdbId}",
                    collection.Title.SanitizeForLog(), collection.TmdbId?.SanitizeForLog());
                return existing;
            }
        }

        collection.Added = DateTime.UtcNow;
        collection.Monitored = true;

        var added = _collectionRepository.Insert(collection);
        _logger.LogInformation("Added collection: {CollectionTitle} - TMDB ID: {TmdbId}",
            added.Title.SanitizeForLog(), added.TmdbId?.SanitizeForLog());

        return added;
    }

    private void ValidateCollection(Collection collection)
    {
        if (string.IsNullOrWhiteSpace(collection.Title))
        {
            throw new ArgumentException("Collection title is required", nameof(collection));
        }

        if (collection.QualityProfileId <= 0)
        {
            throw new ArgumentException("Quality profile ID must be set", nameof(collection));
        }
    }
}

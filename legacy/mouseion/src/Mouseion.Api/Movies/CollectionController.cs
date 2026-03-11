// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Api.Common;
using Mouseion.Core.Movies;

namespace Mouseion.Api.Movies;

[ApiController]
[Route("api/v3/collections")]
[Authorize]
public class CollectionController : ControllerBase
{
    private readonly ICollectionRepository _collectionRepository;
    private readonly IAddCollectionService _addCollectionService;
    private readonly ICollectionStatisticsService _collectionStatisticsService;

    public CollectionController(
        ICollectionRepository collectionRepository,
        IAddCollectionService addCollectionService,
        ICollectionStatisticsService collectionStatisticsService)
    {
        _collectionRepository = collectionRepository;
        _addCollectionService = addCollectionService;
        _collectionStatisticsService = collectionStatisticsService;
    }

    [HttpGet]
    public async Task<ActionResult<PagedResult<CollectionResource>>> GetCollections(
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        CancellationToken ct = default)
    {
        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        var totalCount = await _collectionRepository.CountAsync(ct).ConfigureAwait(false);
        var collections = await _collectionRepository.GetPageAsync(page, pageSize, ct).ConfigureAwait(false);

        return Ok(new PagedResult<CollectionResource>
        {
            Items = collections.Select(ToResource),
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<CollectionResource>> GetCollection(int id, CancellationToken ct = default)
    {
        var collection = await _collectionRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (collection == null)
        {
            return NotFound(new { error = $"Collection {id} not found" });
        }

        return Ok(ToResource(collection));
    }

    [HttpGet("tmdb/{tmdbId}")]
    public async Task<ActionResult<CollectionResource>> GetByTmdbId(string tmdbId, CancellationToken ct = default)
    {
        var collection = await _collectionRepository.FindByTmdbIdAsync(tmdbId, ct).ConfigureAwait(false);
        if (collection == null)
        {
            return NotFound(new { error = $"Collection with TMDB ID {tmdbId} not found" });
        }

        return Ok(ToResource(collection));
    }

    [HttpGet("{id:int}/statistics")]
    public async Task<ActionResult<CollectionStatistics>> GetStatistics(int id, CancellationToken ct = default)
    {
        var stats = await _collectionStatisticsService.GetStatisticsAsync(id, ct).ConfigureAwait(false);
        return Ok(stats);
    }

    [HttpPost]
    public async Task<ActionResult<CollectionResource>> AddCollection([FromBody][Required] CollectionResource resource, CancellationToken ct = default)
    {
        var collection = ToModel(resource);
        var added = await _addCollectionService.AddCollectionAsync(collection, ct).ConfigureAwait(false);
        return CreatedAtAction(nameof(GetCollection), new { id = added.Id }, ToResource(added));
    }

    [HttpPut("{id:int}")]
    public async Task<ActionResult<CollectionResource>> UpdateCollection(int id, [FromBody][Required] CollectionResource resource, CancellationToken ct = default)
    {
        var collection = await _collectionRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (collection == null)
        {
            return NotFound(new { error = $"Collection {id} not found" });
        }

        collection.Title = resource.Title;
        collection.TmdbId = resource.TmdbId;
        collection.Overview = resource.Overview;
        collection.Images = resource.Images ?? new List<string>();
        collection.Monitored = resource.Monitored;
        collection.QualityProfileId = resource.QualityProfileId;

        var updated = await _collectionRepository.UpdateAsync(collection, ct).ConfigureAwait(false);
        return Ok(ToResource(updated));
    }

    [HttpDelete("{id:int}")]
    public async Task<IActionResult> DeleteCollection(int id, CancellationToken ct = default)
    {
        var collection = await _collectionRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (collection == null)
        {
            return NotFound(new { error = $"Collection {id} not found" });
        }

        await _collectionRepository.DeleteAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    private static CollectionResource ToResource(Collection collection)
    {
        return new CollectionResource
        {
            Id = collection.Id,
            Title = collection.Title,
            TmdbId = collection.TmdbId,
            Overview = collection.Overview,
            Images = collection.Images,
            Monitored = collection.Monitored,
            QualityProfileId = collection.QualityProfileId,
            Added = collection.Added
        };
    }

    private static Collection ToModel(CollectionResource resource)
    {
        return new Collection
        {
            Id = resource.Id,
            Title = resource.Title,
            TmdbId = resource.TmdbId,
            Overview = resource.Overview,
            Images = resource.Images ?? new List<string>(),
            Monitored = resource.Monitored,
            QualityProfileId = resource.QualityProfileId,
            Added = resource.Added
        };
    }
}

public class CollectionResource
{
    public int Id { get; set; }
    public string Title { get; set; } = null!;
    public string? TmdbId { get; set; }
    public string? Overview { get; set; }
    public List<string>? Images { get; set; }
    public bool Monitored { get; set; }
    public int QualityProfileId { get; set; }
    public DateTime Added { get; set; }
}

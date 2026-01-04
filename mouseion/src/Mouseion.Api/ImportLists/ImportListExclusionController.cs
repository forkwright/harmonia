// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.ImportLists.ImportExclusions;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Api.ImportLists;

public class ImportListExclusionResource
{
    public int Id { get; set; }
    public MediaType MediaType { get; set; }
    public string Title { get; set; } = string.Empty;
    public int Year { get; set; }
    public int TmdbId { get; set; }
    public string? ImdbId { get; set; }
    public int TvdbId { get; set; }
    public long GoodreadsId { get; set; }
    public string? Isbn { get; set; }
    public Guid MusicBrainzId { get; set; }
    public string? Asin { get; set; }
}

[ApiController]
[Route("api/v3/importlistexclusion")]
public class ImportListExclusionController : ControllerBase
{
    private readonly IImportListExclusionService _service;

    public ImportListExclusionController(IImportListExclusionService service)
    {
        _service = service;
    }

    [HttpGet]
    public ActionResult<List<ImportListExclusionResource>> GetAll()
    {
        var exclusions = _service.GetAll();
        return Ok(exclusions.Select(ToResource).ToList());
    }

    [HttpPost]
    public ActionResult<ImportListExclusionResource> Create([FromBody] ImportListExclusionResource resource)
    {
        var exclusion = new ImportListExclusion
        {
            MediaType = resource.MediaType,
            Title = resource.Title,
            Year = resource.Year,
            TmdbId = resource.TmdbId,
            ImdbId = resource.ImdbId,
            TvdbId = resource.TvdbId,
            GoodreadsId = resource.GoodreadsId,
            Isbn = resource.Isbn,
            MusicBrainzId = resource.MusicBrainzId,
            Asin = resource.Asin
        };

        var created = _service.Add(exclusion);
        return Ok(ToResource(created));
    }

    [HttpDelete("{id}")]
    public IActionResult Delete(int id)
    {
        _service.Delete(id);
        return NoContent();
    }

    private static ImportListExclusionResource ToResource(ImportListExclusion exclusion)
    {
        return new ImportListExclusionResource
        {
            Id = exclusion.Id,
            MediaType = exclusion.MediaType,
            Title = exclusion.Title,
            Year = exclusion.Year,
            TmdbId = exclusion.TmdbId,
            ImdbId = exclusion.ImdbId,
            TvdbId = exclusion.TvdbId,
            GoodreadsId = exclusion.GoodreadsId,
            Isbn = exclusion.Isbn,
            MusicBrainzId = exclusion.MusicBrainzId,
            Asin = exclusion.Asin
        };
    }
}

// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.MediaTypes;
using Mouseion.Core.RootFolders;

namespace Mouseion.Api.RootFolders;

[ApiController]
[Route("api/v3/rootfolders")]
[Authorize]
public class RootFolderController : ControllerBase
{
    private readonly IRootFolderService _rootFolderService;

    public RootFolderController(IRootFolderService rootFolderService)
    {
        _rootFolderService = rootFolderService;
    }

    [HttpGet]
    public async Task<ActionResult<List<RootFolderResource>>> GetRootFolders(
        [FromQuery] int? mediaType = null,
        CancellationToken ct = default)
    {
        var rootFolders = mediaType.HasValue
            ? await _rootFolderService.GetByMediaTypeAsync((MediaType)mediaType.Value, ct).ConfigureAwait(false)
            : await _rootFolderService.GetAllAsync(ct).ConfigureAwait(false);

        return Ok(rootFolders.Select(ToResource));
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<RootFolderResource>> GetRootFolder(int id, CancellationToken ct = default)
    {
        var rootFolder = await _rootFolderService.GetByIdAsync(id, ct).ConfigureAwait(false);
        if (rootFolder == null)
        {
            return NotFound(new { error = $"Root folder {id} not found" });
        }

        return Ok(ToResource(rootFolder));
    }

    [HttpPost]
    public async Task<ActionResult<RootFolderResource>> AddRootFolder(
        [FromBody][Required] RootFolderResource resource,
        CancellationToken ct = default)
    {
        var rootFolder = ToModel(resource);
        var added = await _rootFolderService.AddAsync(rootFolder, ct).ConfigureAwait(false);
        return CreatedAtAction(nameof(GetRootFolder), new { id = added.Id }, ToResource(added));
    }

    [HttpDelete("{id:int}")]
    public async Task<IActionResult> DeleteRootFolder(int id, CancellationToken ct = default)
    {
        var rootFolder = await _rootFolderService.GetByIdAsync(id, ct).ConfigureAwait(false);
        if (rootFolder == null)
        {
            return NotFound(new { error = $"Root folder {id} not found" });
        }

        await _rootFolderService.DeleteAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    [HttpPost("{id:int}/refresh")]
    public async Task<ActionResult<RootFolderResource>> RefreshDiskSpace(int id, CancellationToken ct = default)
    {
        await _rootFolderService.UpdateFreeSpaceAsync(id, ct).ConfigureAwait(false);
        var rootFolder = await _rootFolderService.GetByIdAsync(id, ct).ConfigureAwait(false);
        if (rootFolder == null)
        {
            return NotFound(new { error = $"Root folder {id} not found" });
        }

        return Ok(ToResource(rootFolder));
    }

    private static RootFolderResource ToResource(RootFolder rootFolder)
    {
        return new RootFolderResource
        {
            Id = rootFolder.Id,
            Path = rootFolder.Path,
            MediaType = (int)rootFolder.MediaType,
            Accessible = rootFolder.Accessible,
            FreeSpace = rootFolder.FreeSpace,
            TotalSpace = rootFolder.TotalSpace
        };
    }

    private static RootFolder ToModel(RootFolderResource resource)
    {
        return new RootFolder
        {
            Id = resource.Id,
            Path = resource.Path,
            MediaType = (MediaType)resource.MediaType,
            Accessible = resource.Accessible,
            FreeSpace = resource.FreeSpace,
            TotalSpace = resource.TotalSpace
        };
    }
}

public class RootFolderResource
{
    public int Id { get; set; }
    public string Path { get; set; } = string.Empty;
    public int MediaType { get; set; }
    public bool Accessible { get; set; }
    public long? FreeSpace { get; set; }
    public long? TotalSpace { get; set; }
}

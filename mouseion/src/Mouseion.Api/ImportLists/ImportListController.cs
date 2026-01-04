// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.ImportLists;

namespace Mouseion.Api.ImportLists;

[ApiController]
[Route("api/v3/importlist")]
public class ImportListController : ControllerBase
{
    private readonly IImportListRepository _repository;
    private readonly IImportListFactory _factory;
    private readonly IImportListSyncService _syncService;

    public ImportListController(IImportListRepository repository, IImportListFactory factory, IImportListSyncService syncService)
    {
        _repository = repository;
        _factory = factory;
        _syncService = syncService;
    }

    [HttpGet]
    public ActionResult<List<ImportListResource>> GetAll()
    {
        var definitions = _repository.All();
        return Ok(definitions.Select(d => d.ToResource()).ToList());
    }

    [HttpGet("{id}")]
    public ActionResult<ImportListResource> GetById(int id)
    {
        var definition = _repository.Get(id);
        return Ok(definition.ToResource());
    }

    [HttpPost]
    public ActionResult<ImportListResource> Create([FromBody] ImportListResource resource)
    {
        var definition = resource.ToDefinition();
        var created = _repository.Insert(definition);
        return CreatedAtAction(nameof(GetById), new { id = created.Id }, created.ToResource());
    }

    [HttpPut("{id}")]
    public ActionResult<ImportListResource> Update(int id, [FromBody] ImportListResource resource)
    {
        resource.Id = id;
        var definition = resource.ToDefinition();
        var updated = _repository.Update(definition);
        return Ok(updated.ToResource());
    }

    [HttpDelete("{id}")]
    public IActionResult Delete(int id)
    {
        _repository.Delete(id);
        return NoContent();
    }

    [HttpPost("test")]
    public async Task<ActionResult> Test([FromBody] ImportListResource resource)
    {
        var list = _factory.Get(resource.Id);
        var success = await list.TestAsync();
        return Ok(new { success });
    }

    [HttpPost("sync")]
    public async Task<ActionResult<List<ImportListItemResource>>> SyncAll()
    {
        var result = await _syncService.SyncAllAsync();
        return Ok(new
        {
            items = result.Items.Select(i => i.ToResource()).ToList(),
            syncedLists = result.SyncedLists,
            anyFailure = result.AnyFailure
        });
    }

    [HttpPost("sync/{id}")]
    public async Task<ActionResult<List<ImportListItemResource>>> SyncList(int id)
    {
        var result = await _syncService.SyncListAsync(id);
        return Ok(new
        {
            items = result.Items.Select(i => i.ToResource()).ToList(),
            syncedLists = result.SyncedLists,
            anyFailure = result.AnyFailure
        });
    }
}

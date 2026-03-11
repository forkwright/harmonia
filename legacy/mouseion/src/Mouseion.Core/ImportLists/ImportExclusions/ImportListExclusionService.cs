// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;

namespace Mouseion.Core.ImportLists.ImportExclusions;

public interface IImportListExclusionService
{
    List<ImportListExclusion> GetAll();
    ImportListExclusion Add(ImportListExclusion exclusion);
    void Delete(int id);
}

public class ImportListExclusionService : IImportListExclusionService
{
    private readonly IImportListExclusionRepository _repository;
    private readonly ILogger<ImportListExclusionService> _logger;

    public ImportListExclusionService(IImportListExclusionRepository repository, ILogger<ImportListExclusionService> logger)
    {
        _repository = repository;
        _logger = logger;
    }

    public List<ImportListExclusion> GetAll() => _repository.All().ToList();

    public ImportListExclusion Add(ImportListExclusion exclusion)
    {
        _logger.LogInformation("Adding import list exclusion for {Title} ({Year})", exclusion.Title, exclusion.Year);
        return _repository.Insert(exclusion);
    }

    public void Delete(int id)
    {
        _logger.LogInformation("Deleting import list exclusion {Id}", id);
        _repository.Delete(id);
    }
}

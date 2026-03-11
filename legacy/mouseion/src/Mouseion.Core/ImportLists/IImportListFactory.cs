// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.ImportLists;

public interface IImportListFactory
{
    IImportList Get(int id);
    List<IImportList> GetEnabled();
    List<IImportList> GetAll();
}

public class ImportListFactory : IImportListFactory
{
    private readonly IImportListRepository _repository;
    private readonly IEnumerable<IImportList> _importLists;

    public ImportListFactory(IImportListRepository repository, IEnumerable<IImportList> importLists)
    {
        _repository = repository;
        _importLists = importLists;
    }

    public IImportList Get(int id)
    {
        var definition = _repository.Get(id);
        var implementation = _importLists.FirstOrDefault(x => x.GetType().Name == definition.Implementation)
            ?? throw new InvalidOperationException($"Import list implementation {definition.Implementation} not found");
        implementation.Definition = definition;
        return implementation;
    }

    public List<IImportList> GetEnabled()
    {
        var definitions = _repository.GetEnabled();
        return definitions.Select(def =>
        {
            var impl = _importLists.FirstOrDefault(x => x.GetType().Name == def.Implementation);
            if (impl != null) impl.Definition = def;
            return impl;
        }).Where(x => x != null).Cast<IImportList>().ToList();
    }

    public List<IImportList> GetAll()
    {
        var definitions = _repository.All();
        return definitions.Select(def =>
        {
            var impl = _importLists.FirstOrDefault(x => x.GetType().Name == def.Implementation);
            if (impl != null) impl.Definition = def;
            return impl;
        }).Where(x => x != null).Cast<IImportList>().ToList();
    }
}

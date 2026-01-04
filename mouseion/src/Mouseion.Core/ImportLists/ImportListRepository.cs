// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.ImportLists;

public interface IImportListRepository : IBasicRepository<ImportListDefinition>
{
    List<ImportListDefinition> GetEnabled();
}

public class ImportListRepository : BasicRepository<ImportListDefinition>, IImportListRepository
{
    public ImportListRepository(IDatabase database) : base(database) { }

    public List<ImportListDefinition> GetEnabled() => All().Where(x => x.Enabled).ToList();
}

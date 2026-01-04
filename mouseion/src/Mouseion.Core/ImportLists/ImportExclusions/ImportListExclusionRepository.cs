// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.ImportLists.ImportExclusions;

public interface IImportListExclusionRepository : IBasicRepository<ImportListExclusion> { }

public class ImportListExclusionRepository : BasicRepository<ImportListExclusion>, IImportListExclusionRepository
{
    public ImportListExclusionRepository(IDatabase database) : base(database) { }
}

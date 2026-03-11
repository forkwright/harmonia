// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.ImportLists;

public interface IImportList
{
    string Name { get; }
    ImportListType ListType { get; }
    TimeSpan MinRefreshInterval { get; }
    bool Enabled { get; }
    bool EnableAuto { get; }
    ImportListDefinition Definition { get; set; }
    Task<ImportListFetchResult> FetchAsync(CancellationToken cancellationToken = default);
    Task<bool> TestAsync(CancellationToken cancellationToken = default);
}

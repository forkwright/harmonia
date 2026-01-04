// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.ImportLists;

public class ImportListFetchResult
{
    public List<ImportListItem> Items { get; set; } = new();
    public bool AnyFailure { get; set; }
    public int SyncedLists { get; set; }
}

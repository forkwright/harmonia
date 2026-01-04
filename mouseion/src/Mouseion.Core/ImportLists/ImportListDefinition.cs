// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.ImportLists;

public class ImportListDefinition : ModelBase
{
    public string Name { get; set; } = string.Empty;
    public bool Enabled { get; set; }
    public bool EnableAuto { get; set; }
    public string Implementation { get; set; } = string.Empty;
    public string ConfigContract { get; set; } = string.Empty;
    public string Settings { get; set; } = string.Empty;
    public ImportListType ListType { get; set; }
    public MediaType MediaType { get; set; }
    public TimeSpan MinRefreshInterval { get; set; }
    public bool Monitor { get; set; }
    public int QualityProfileId { get; set; }
    public string RootFolderPath { get; set; } = string.Empty;
    public bool SearchOnAdd { get; set; }
    public HashSet<int> Tags { get; set; } = new();
}

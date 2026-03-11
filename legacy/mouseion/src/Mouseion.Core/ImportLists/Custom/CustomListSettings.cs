// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.ImportLists.Custom;

public class CustomListSettings : ImportListSettingsBase
{
    public MediaType MediaType { get; set; } = MediaType.Movie;
    public List<CustomListEntry> Entries { get; set; } = new();
}

public class CustomListEntry
{
    public string Title { get; set; } = string.Empty;
    public int Year { get; set; }
    public int TmdbId { get; set; }
    public string? ImdbId { get; set; }
}

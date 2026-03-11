// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.ImportLists.RSS;

public class RssImportSettings : ImportListSettingsBase
{
    public string FeedUrl { get; set; } = string.Empty;
    public MediaType MediaType { get; set; } = MediaType.Movie;
}

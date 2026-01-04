// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.ImportLists.ImportExclusions;

public class ImportListExclusion : ModelBase
{
    public MediaType MediaType { get; set; }
    public string Title { get; set; } = string.Empty;
    public int Year { get; set; }
    public int TmdbId { get; set; }
    public string? ImdbId { get; set; }
    public int TvdbId { get; set; }
    public long GoodreadsId { get; set; }
    public string? Isbn { get; set; }
    public Guid MusicBrainzId { get; set; }
    public string? Asin { get; set; }
}

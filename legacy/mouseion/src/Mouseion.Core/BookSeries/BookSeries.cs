// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.BookSeries;

public class BookSeries : ModelBase
{
    public string Title { get; set; } = null!;
    public string? SortTitle { get; set; }
    public string? Description { get; set; }
    public string? ForeignSeriesId { get; set; }
    public int? AuthorId { get; set; }
    public bool Monitored { get; set; }

    public override string ToString()
    {
        return Title;
    }
}

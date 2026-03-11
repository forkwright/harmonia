// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.TV;

public class Season : ModelBase
{
    public int SeriesId { get; set; }
    public int SeasonNumber { get; set; }
    public bool Monitored { get; set; } = true;

    public override string ToString()
    {
        return $"Season {SeasonNumber}";
    }
}

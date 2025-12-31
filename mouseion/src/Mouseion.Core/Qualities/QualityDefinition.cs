// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Qualities;

public class QualityDefinition : ModelBase
{
    public Quality Quality { get; set; }

    public string Title { get; set; }
    public string GroupName { get; set; }
    public int Weight { get; set; }

    public double? MinSize { get; set; }
    public double? MaxSize { get; set; }
    public double? PreferredSize { get; set; }

    public QualityDefinition()
    {
    }

    public QualityDefinition(Quality quality)
    {
        Quality = quality;
        Title = quality.Name;
    }

    public override string ToString()
    {
        return Quality.Name;
    }
}

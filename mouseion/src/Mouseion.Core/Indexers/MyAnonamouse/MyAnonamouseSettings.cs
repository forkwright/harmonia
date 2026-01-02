// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Indexers.MyAnonamouse;

public class MyAnonamouseSettings : IndexerSettings
{
    public MyAnonamouseSettings()
    {
        BaseUrl = "https://www.myanonamouse.net";
        MinimumSeeders = 1;
        SearchType = MyAnonamouseSearchType.All;
    }

    public string MamId { get; set; } = string.Empty;
    public MyAnonamouseSearchType SearchType { get; set; }
    public bool SearchInDescription { get; set; }
    public bool SearchInSeries { get; set; }
    public bool SearchInFilenames { get; set; }
}

public enum MyAnonamouseSearchType
{
    All = 0,
    Active = 1,
    Freeleech = 2,
    FreeleechOrVip = 3,
    Vip = 4,
    NotVip = 5
}

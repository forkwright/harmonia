// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Collections.Generic;

namespace Mouseion.Common.Cache
{
    public interface ICacheManager
    {
        ICached<T> GetCache<T>(Type host);
        ICached<T> GetCache<T>(Type host, string name);
        ICached<T> GetRollingCache<T>(Type host, string name, TimeSpan defaultLifeTime);
        ICollection<ICached> Caches { get; }
    }
}

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
    public interface ICached
    {
        void Clear();
        void ClearExpired();
        void Remove(string key);
    }

    public interface ICached<T> : ICached
    {
        void Set(string key, T value, TimeSpan? lifetime = null);
        T Get(string key, Func<T> function, TimeSpan? lifeTime = null);
        T Find(string key);
        bool ContainsKey(string key);
        ICollection<T> Values { get; }
        ICollection<string> Keys { get; }
    }
}

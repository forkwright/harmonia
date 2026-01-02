// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;

namespace Mouseion.Common.TPL
{
    public interface IDebounceManager
    {
        Debouncer CreateDebouncer(Action action, TimeSpan debounceDuration);
    }

    public class DebounceManager : IDebounceManager
    {
        public Debouncer CreateDebouncer(Action action, TimeSpan debounceDuration)
        {
            return new Debouncer(action, debounceDuration);
        }
    }
}

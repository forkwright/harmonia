// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Common.EnsureThat
{
    public abstract class Param
    {
        public const string DefaultName = "";

        public readonly string Name;

        protected Param(string name)
        {
            Name = name;
        }
    }

    public class Param<T> : Param
    {
        public readonly T Value;

        internal Param(string name, T value)
            : base(name)
        {
            Value = value;
        }
    }
}

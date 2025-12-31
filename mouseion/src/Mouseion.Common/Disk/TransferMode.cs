// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;

namespace Mouseion.Common.Disk
{
    [Flags]
    public enum TransferMode
    {
        None = 0,

        Move = 1,
        Copy = 2,
        HardLink = 4,

        HardLinkOrCopy = Copy | HardLink
    }
}

// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;

namespace Mouseion.Common.Http
{
    public class BasicNetworkCredential : NetworkCredential
    {
        public BasicNetworkCredential(string user, string pass)
        : base(user, pass)
        {
        }
    }
}

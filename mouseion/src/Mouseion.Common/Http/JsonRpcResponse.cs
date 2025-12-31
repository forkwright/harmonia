// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Newtonsoft.Json.Linq;

namespace Mouseion.Common.Http
{
    public class JsonRpcResponse<T>
    {
        public string Id { get; set; }
        public T Result { get; set; }
        public JToken Error { get; set; }
    }
}

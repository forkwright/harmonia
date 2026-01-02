// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Common.Http
{
    public class HttpFormData
    {
        public string Name { get; set; } = string.Empty;
        public string? FileName { get; set; }
        public byte[]? ContentData { get; set; }
        public string? ContentType { get; set; }
    }
}

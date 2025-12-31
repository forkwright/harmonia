// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Common.Http
{
    public interface IHttpRequestInterceptor
    {
        HttpRequest PreRequest(HttpRequest request);
        HttpResponse PostResponse(HttpResponse response);
    }
}

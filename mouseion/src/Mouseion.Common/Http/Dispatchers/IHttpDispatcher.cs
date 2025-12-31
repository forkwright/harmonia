// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Threading.Tasks;

namespace Mouseion.Common.Http.Dispatchers
{
    public interface IHttpDispatcher
    {
        Task<HttpResponse> GetResponseAsync(HttpRequest request, CookieContainer cookies);
    }
}

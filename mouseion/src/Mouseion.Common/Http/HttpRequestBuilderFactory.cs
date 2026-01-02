// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Common.Http
{
    public interface IHttpRequestBuilderFactory
    {
        HttpRequestBuilder Create();
    }

    public class HttpRequestBuilderFactory : IHttpRequestBuilderFactory
    {
        private HttpRequestBuilder _rootBuilder = null!;

        public HttpRequestBuilderFactory(HttpRequestBuilder rootBuilder)
        {
            SetRootBuilder(rootBuilder);
        }

        protected HttpRequestBuilderFactory()
        {
        }

        protected void SetRootBuilder(HttpRequestBuilder rootBuilder)
        {
            _rootBuilder = rootBuilder.Clone();
        }

        public HttpRequestBuilder Create()
        {
            return _rootBuilder.Clone();
        }
    }
}

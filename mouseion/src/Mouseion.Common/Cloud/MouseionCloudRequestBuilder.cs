// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Common.Http;

namespace Mouseion.Common.Cloud
{
    public interface IMouseionCloudRequestBuilder
    {
        IHttpRequestBuilderFactory Services { get; }
        IHttpRequestBuilderFactory TMDB { get; }
        IHttpRequestBuilderFactory MouseionMetadata { get; }
    }

    public class MouseionCloudRequestBuilder : IMouseionCloudRequestBuilder
    {
        public MouseionCloudRequestBuilder()
        {
            Services = new HttpRequestBuilder("https://radarr.servarr.com/v1/")
                .CreateFactory();

            // TMDB API token must be provided via TMDB_API_TOKEN environment variable
            var tmdbToken = Environment.GetEnvironmentVariable("TMDB_API_TOKEN");
            if (string.IsNullOrWhiteSpace(tmdbToken))
            {
                throw new InvalidOperationException(
                    "TMDB_API_TOKEN environment variable is required. " +
                    "Get a free API key from https://www.themoviedb.org/settings/api");
            }

            TMDB = new HttpRequestBuilder("https://api.themoviedb.org/{api}/{route}/{id}{secondaryRoute}")
                .SetHeader("Authorization", $"Bearer {tmdbToken}")
                .CreateFactory();

            MouseionMetadata = new HttpRequestBuilder("https://api.radarr.video/v1/{route}")
                .CreateFactory();
        }

        public IHttpRequestBuilderFactory Services { get; private set; }
        public IHttpRequestBuilderFactory TMDB { get; private set; }
        public IHttpRequestBuilderFactory MouseionMetadata { get; private set; }
    }
}

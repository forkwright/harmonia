// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Podcasts;

namespace Mouseion.Core.MetadataSource;

public interface IProvidePodcastInfo
{
    Task<List<PodcastShow>> SearchAsync(string query, CancellationToken ct = default);
    Task<PodcastShow?> GetByFeedUrlAsync(string feedUrl, CancellationToken ct = default);
    Task<PodcastShow?> GetByPodcastIndexIdAsync(int podcastIndexId, CancellationToken ct = default);
}

// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Podcasts;

public interface IAddPodcastService
{
    Task<PodcastShow> AddPodcastAsync(string feedUrl, string? rootFolderPath = null, int qualityProfileId = 1, bool monitored = true, CancellationToken ct = default);
    PodcastShow AddPodcast(string feedUrl, string? rootFolderPath = null, int qualityProfileId = 1, bool monitored = true);
}

public class AddPodcastService : IAddPodcastService
{
    private readonly IPodcastShowRepository _showRepository;
    private readonly ILogger<AddPodcastService> _logger;

    public AddPodcastService(
        IPodcastShowRepository showRepository,
        ILogger<AddPodcastService> logger)
    {
        _showRepository = showRepository;
        _logger = logger;
    }

    public async Task<PodcastShow> AddPodcastAsync(
        string feedUrl,
        string? rootFolderPath = null,
        int qualityProfileId = 1,
        bool monitored = true,
        CancellationToken ct = default)
    {
        // Check if podcast already exists
        var existing = await _showRepository.FindByFeedUrlAsync(feedUrl, ct).ConfigureAwait(false);
        if (existing != null)
        {
            _logger.LogInformation("Podcast with feed URL {FeedUrl} already exists", feedUrl);
            return existing;
        }

        var show = new PodcastShow
        {
            FeedUrl = feedUrl,
            RootFolderPath = rootFolderPath,
            QualityProfileId = qualityProfileId,
            Monitored = monitored,
            MonitorNewEpisodes = true,
            Added = DateTime.UtcNow
        };

        var inserted = await _showRepository.InsertAsync(show, ct).ConfigureAwait(false);
        _logger.LogInformation("Added podcast show {Title} (ID: {Id})", inserted.Title, inserted.Id);

        return inserted;
    }

    public PodcastShow AddPodcast(
        string feedUrl,
        string? rootFolderPath = null,
        int qualityProfileId = 1,
        bool monitored = true)
    {
        var existing = _showRepository.FindByFeedUrl(feedUrl);
        if (existing != null)
        {
            _logger.LogInformation("Podcast with feed URL {FeedUrl} already exists", feedUrl);
            return existing;
        }

        var show = new PodcastShow
        {
            FeedUrl = feedUrl,
            RootFolderPath = rootFolderPath,
            QualityProfileId = qualityProfileId,
            Monitored = monitored,
            MonitorNewEpisodes = true,
            Added = DateTime.UtcNow
        };

        var inserted = _showRepository.Insert(show);
        _logger.LogInformation("Added podcast show {Title} (ID: {Id})", inserted.Title, inserted.Id);

        return inserted;
    }
}

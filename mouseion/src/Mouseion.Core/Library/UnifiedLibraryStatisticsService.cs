// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Manga;
using Mouseion.Core.News;
using Mouseion.Core.Webcomic;

namespace Mouseion.Core.Library;

public class UnifiedLibraryStatistics
{
    public MediaTypeStatistics Movies { get; set; } = new();
    public MediaTypeStatistics Books { get; set; } = new();
    public MediaTypeStatistics Audiobooks { get; set; } = new();
    public MediaTypeStatistics Music { get; set; } = new();
    public MediaTypeStatistics TV { get; set; } = new();
    public MediaTypeStatistics Podcasts { get; set; } = new();
    public MediaTypeStatistics News { get; set; } = new();
    public MediaTypeStatistics Manga { get; set; } = new();
    public MediaTypeStatistics Webcomics { get; set; } = new();

    public int TotalMediaItems { get; set; }
    public int TotalMonitored { get; set; }
    public int TotalUnread { get; set; }
}

public class MediaTypeStatistics
{
    public int Total { get; set; }
    public int Monitored { get; set; }
    public int Unread { get; set; }
    public int ChildItems { get; set; }
}

public interface IUnifiedLibraryStatisticsService
{
    Task<UnifiedLibraryStatistics> GetStatisticsAsync(CancellationToken ct = default);
}

public class UnifiedLibraryStatisticsService : IUnifiedLibraryStatisticsService
{
    private readonly INewsFeedRepository _newsFeedRepository;
    private readonly INewsArticleRepository _newsArticleRepository;
    private readonly IMangaSeriesRepository _mangaSeriesRepository;
    private readonly IMangaChapterRepository _mangaChapterRepository;
    private readonly IWebcomicSeriesRepository _webcomicSeriesRepository;
    private readonly IWebcomicEpisodeRepository _webcomicEpisodeRepository;

    public UnifiedLibraryStatisticsService(
        INewsFeedRepository newsFeedRepository,
        INewsArticleRepository newsArticleRepository,
        IMangaSeriesRepository mangaSeriesRepository,
        IMangaChapterRepository mangaChapterRepository,
        IWebcomicSeriesRepository webcomicSeriesRepository,
        IWebcomicEpisodeRepository webcomicEpisodeRepository)
    {
        _newsFeedRepository = newsFeedRepository;
        _newsArticleRepository = newsArticleRepository;
        _mangaSeriesRepository = mangaSeriesRepository;
        _mangaChapterRepository = mangaChapterRepository;
        _webcomicSeriesRepository = webcomicSeriesRepository;
        _webcomicEpisodeRepository = webcomicEpisodeRepository;
    }

    public async Task<UnifiedLibraryStatistics> GetStatisticsAsync(CancellationToken ct = default)
    {
        var stats = new UnifiedLibraryStatistics();

        // News statistics
        var newsFeeds = await _newsFeedRepository.AllAsync(ct).ConfigureAwait(false);
        var newsFeedsList = newsFeeds.ToList();
        var unreadArticles = await _newsArticleRepository.GetUnreadCountAsync(ct).ConfigureAwait(false);
        var allArticles = await _newsArticleRepository.AllAsync(ct).ConfigureAwait(false);

        stats.News = new MediaTypeStatistics
        {
            Total = newsFeedsList.Count,
            Monitored = newsFeedsList.Count(f => f.Monitored),
            Unread = unreadArticles,
            ChildItems = allArticles.Count()
        };

        // Manga statistics
        var mangaSeries = await _mangaSeriesRepository.AllAsync(ct).ConfigureAwait(false);
        var mangaSeriesList = mangaSeries.ToList();
        var unreadChapters = await _mangaChapterRepository.GetUnreadCountAsync(ct).ConfigureAwait(false);
        var allChapters = await _mangaChapterRepository.AllAsync(ct).ConfigureAwait(false);

        stats.Manga = new MediaTypeStatistics
        {
            Total = mangaSeriesList.Count,
            Monitored = mangaSeriesList.Count(s => s.Monitored),
            Unread = unreadChapters,
            ChildItems = allChapters.Count()
        };

        // Webcomic statistics
        var webcomicSeries = await _webcomicSeriesRepository.AllAsync(ct).ConfigureAwait(false);
        var webcomicSeriesList = webcomicSeries.ToList();
        var unreadEpisodes = await _webcomicEpisodeRepository.GetUnreadCountAsync(ct).ConfigureAwait(false);
        var allEpisodes = await _webcomicEpisodeRepository.AllAsync(ct).ConfigureAwait(false);

        stats.Webcomics = new MediaTypeStatistics
        {
            Total = webcomicSeriesList.Count,
            Monitored = webcomicSeriesList.Count(s => s.Monitored),
            Unread = unreadEpisodes,
            ChildItems = allEpisodes.Count()
        };

        // Aggregate totals
        stats.TotalMediaItems = stats.News.Total + stats.Manga.Total + stats.Webcomics.Total;
        stats.TotalMonitored = stats.News.Monitored + stats.Manga.Monitored + stats.Webcomics.Monitored;
        stats.TotalUnread = stats.News.Unread + stats.Manga.Unread + stats.Webcomics.Unread;

        return stats;
    }
}

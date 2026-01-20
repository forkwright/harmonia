// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Webcomic;

public interface IAddWebcomicSeriesService
{
    Task<WebcomicSeries> AddSeriesAsync(WebcomicSeries series, CancellationToken ct = default);
    Task<WebcomicSeries?> FindByWebtoonIdAsync(string webtoonId, CancellationToken ct = default);
    Task<WebcomicSeries?> FindByTapasIdAsync(string tapasId, CancellationToken ct = default);
}

public class AddWebcomicSeriesService : IAddWebcomicSeriesService
{
    private readonly IWebcomicSeriesRepository _seriesRepository;
    private readonly ILogger<AddWebcomicSeriesService> _logger;

    public AddWebcomicSeriesService(
        IWebcomicSeriesRepository seriesRepository,
        ILogger<AddWebcomicSeriesService> logger)
    {
        _seriesRepository = seriesRepository;
        _logger = logger;
    }

    public async Task<WebcomicSeries> AddSeriesAsync(WebcomicSeries series, CancellationToken ct = default)
    {
        if (!string.IsNullOrEmpty(series.WebtoonId))
        {
            var existing = await _seriesRepository.FindByWebtoonIdAsync(series.WebtoonId, ct).ConfigureAwait(false);
            if (existing != null)
            {
                _logger.LogInformation("Webcomic series with Webtoon ID {WebtoonId} already exists", series.WebtoonId);
                return existing;
            }
        }

        if (!string.IsNullOrEmpty(series.TapasId))
        {
            var existing = await _seriesRepository.FindByTapasIdAsync(series.TapasId, ct).ConfigureAwait(false);
            if (existing != null)
            {
                _logger.LogInformation("Webcomic series with Tapas ID {TapasId} already exists", series.TapasId);
                return existing;
            }
        }

        series.Added = DateTime.UtcNow;
        series.SortTitle ??= series.Title.ToLowerInvariant();

        var insertedSeries = await _seriesRepository.InsertAsync(series, ct).ConfigureAwait(false);
        _logger.LogInformation("Added webcomic series {Title} (ID: {Id})", insertedSeries.Title, insertedSeries.Id);

        return insertedSeries;
    }

    public async Task<WebcomicSeries?> FindByWebtoonIdAsync(string webtoonId, CancellationToken ct = default)
    {
        return await _seriesRepository.FindByWebtoonIdAsync(webtoonId, ct).ConfigureAwait(false);
    }

    public async Task<WebcomicSeries?> FindByTapasIdAsync(string tapasId, CancellationToken ct = default)
    {
        return await _seriesRepository.FindByTapasIdAsync(tapasId, ct).ConfigureAwait(false);
    }
}

// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.Datastore;
using Mouseion.Core.Movies;
using Mouseion.Core.Books;
using Mouseion.Core.Audiobooks;
using Mouseion.Core.Music;
using Mouseion.Core.TV;
using Mouseion.Core.Manga;
using Mouseion.Core.Webcomic;
using Mouseion.Core.Comic;
using Mouseion.Core.News;

namespace Mouseion.Core.Bulk;

public interface IBulkOperationsService
{
    Task<BulkUpdateResult> UpdateMoviesAsync(BulkUpdateRequest request, CancellationToken ct = default);
    Task<BulkDeleteResult> DeleteMoviesAsync(BulkDeleteRequest request, CancellationToken ct = default);

    Task<BulkUpdateResult> UpdateBooksAsync(BulkUpdateRequest request, CancellationToken ct = default);
    Task<BulkDeleteResult> DeleteBooksAsync(BulkDeleteRequest request, CancellationToken ct = default);

    Task<BulkUpdateResult> UpdateAudiobooksAsync(BulkUpdateRequest request, CancellationToken ct = default);
    Task<BulkDeleteResult> DeleteAudiobooksAsync(BulkDeleteRequest request, CancellationToken ct = default);

    Task<BulkUpdateResult> UpdateSeriesAsync(BulkUpdateRequest request, CancellationToken ct = default);
    Task<BulkDeleteResult> DeleteSeriesAsync(BulkDeleteRequest request, CancellationToken ct = default);

    Task<BulkReadResult> MarkMangaChaptersReadAsync(BulkReadRequest request, CancellationToken ct = default);
    Task<BulkReadResult> MarkWebcomicEpisodesReadAsync(BulkReadRequest request, CancellationToken ct = default);
    Task<BulkReadResult> MarkComicIssuesReadAsync(BulkReadRequest request, CancellationToken ct = default);
    Task<BulkReadResult> MarkArticlesReadAsync(BulkReadRequest request, CancellationToken ct = default);
}

public class BulkOperationsService : IBulkOperationsService
{
    private const int MaxBatchSize = 100;

    private readonly ILogger<BulkOperationsService> _logger;
    private readonly IMovieRepository _movieRepository;
    private readonly IBookRepository _bookRepository;
    private readonly IAudiobookRepository _audiobookRepository;
    private readonly ISeriesRepository _seriesRepository;
    private readonly IMangaChapterRepository _mangaChapterRepository;
    private readonly IWebcomicEpisodeRepository _webcomicEpisodeRepository;
    private readonly IComicIssueRepository _comicIssueRepository;
    private readonly INewsArticleRepository _newsArticleRepository;

    public BulkOperationsService(
        ILogger<BulkOperationsService> logger,
        IMovieRepository movieRepository,
        IBookRepository bookRepository,
        IAudiobookRepository audiobookRepository,
        ISeriesRepository seriesRepository,
        IMangaChapterRepository mangaChapterRepository,
        IWebcomicEpisodeRepository webcomicEpisodeRepository,
        IComicIssueRepository comicIssueRepository,
        INewsArticleRepository newsArticleRepository)
    {
        _logger = logger;
        _movieRepository = movieRepository;
        _bookRepository = bookRepository;
        _audiobookRepository = audiobookRepository;
        _seriesRepository = seriesRepository;
        _mangaChapterRepository = mangaChapterRepository;
        _webcomicEpisodeRepository = webcomicEpisodeRepository;
        _comicIssueRepository = comicIssueRepository;
        _newsArticleRepository = newsArticleRepository;
    }

    public async Task<BulkUpdateResult> UpdateMoviesAsync(BulkUpdateRequest request, CancellationToken ct = default)
    {
        ValidateBatchSize(request.Items.Count);
        var result = new BulkUpdateResult();

        foreach (var item in request.Items)
        {
            try
            {
                var movie = await _movieRepository.FindAsync(item.Id, ct).ConfigureAwait(false);
                if (movie == null)
                {
                    AddError(result, item.Id, $"Movie with ID {item.Id} not found");
                    continue;
                }

                ApplyCommonUpdates(movie, item);
                await _movieRepository.UpdateAsync(movie, ct).ConfigureAwait(false);
                result.UpdatedIds.Add(item.Id);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to update movie {Id}", item.Id);
                AddError(result, item.Id, ex.Message);
            }
        }

        result.Updated = result.UpdatedIds.Count;
        return result;
    }

    public async Task<BulkDeleteResult> DeleteMoviesAsync(BulkDeleteRequest request, CancellationToken ct = default)
    {
        ValidateBatchSize(request.Ids.Count);
        var result = new BulkDeleteResult();

        foreach (var id in request.Ids)
        {
            try
            {
                var movie = await _movieRepository.FindAsync(id, ct).ConfigureAwait(false);
                if (movie == null)
                {
                    AddError(result, id, $"Movie with ID {id} not found");
                    continue;
                }

                await _movieRepository.DeleteAsync(id, ct).ConfigureAwait(false);
                result.DeletedIds.Add(id);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to delete movie {Id}", id);
                AddError(result, id, ex.Message);
            }
        }

        result.Deleted = result.DeletedIds.Count;
        return result;
    }

    public async Task<BulkUpdateResult> UpdateBooksAsync(BulkUpdateRequest request, CancellationToken ct = default)
    {
        ValidateBatchSize(request.Items.Count);
        var result = new BulkUpdateResult();

        foreach (var item in request.Items)
        {
            try
            {
                var book = await _bookRepository.FindAsync(item.Id, ct).ConfigureAwait(false);
                if (book == null)
                {
                    AddError(result, item.Id, $"Book with ID {item.Id} not found");
                    continue;
                }

                ApplyCommonUpdates(book, item);
                await _bookRepository.UpdateAsync(book, ct).ConfigureAwait(false);
                result.UpdatedIds.Add(item.Id);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to update book {Id}", item.Id);
                AddError(result, item.Id, ex.Message);
            }
        }

        result.Updated = result.UpdatedIds.Count;
        return result;
    }

    public async Task<BulkDeleteResult> DeleteBooksAsync(BulkDeleteRequest request, CancellationToken ct = default)
    {
        ValidateBatchSize(request.Ids.Count);
        var result = new BulkDeleteResult();

        foreach (var id in request.Ids)
        {
            try
            {
                var book = await _bookRepository.FindAsync(id, ct).ConfigureAwait(false);
                if (book == null)
                {
                    AddError(result, id, $"Book with ID {id} not found");
                    continue;
                }

                await _bookRepository.DeleteAsync(id, ct).ConfigureAwait(false);
                result.DeletedIds.Add(id);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to delete book {Id}", id);
                AddError(result, id, ex.Message);
            }
        }

        result.Deleted = result.DeletedIds.Count;
        return result;
    }

    public async Task<BulkUpdateResult> UpdateAudiobooksAsync(BulkUpdateRequest request, CancellationToken ct = default)
    {
        ValidateBatchSize(request.Items.Count);
        var result = new BulkUpdateResult();

        foreach (var item in request.Items)
        {
            try
            {
                var audiobook = await _audiobookRepository.FindAsync(item.Id, ct).ConfigureAwait(false);
                if (audiobook == null)
                {
                    AddError(result, item.Id, $"Audiobook with ID {item.Id} not found");
                    continue;
                }

                ApplyCommonUpdates(audiobook, item);
                await _audiobookRepository.UpdateAsync(audiobook, ct).ConfigureAwait(false);
                result.UpdatedIds.Add(item.Id);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to update audiobook {Id}", item.Id);
                AddError(result, item.Id, ex.Message);
            }
        }

        result.Updated = result.UpdatedIds.Count;
        return result;
    }

    public async Task<BulkDeleteResult> DeleteAudiobooksAsync(BulkDeleteRequest request, CancellationToken ct = default)
    {
        ValidateBatchSize(request.Ids.Count);
        var result = new BulkDeleteResult();

        foreach (var id in request.Ids)
        {
            try
            {
                var audiobook = await _audiobookRepository.FindAsync(id, ct).ConfigureAwait(false);
                if (audiobook == null)
                {
                    AddError(result, id, $"Audiobook with ID {id} not found");
                    continue;
                }

                await _audiobookRepository.DeleteAsync(id, ct).ConfigureAwait(false);
                result.DeletedIds.Add(id);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to delete audiobook {Id}", id);
                AddError(result, id, ex.Message);
            }
        }

        result.Deleted = result.DeletedIds.Count;
        return result;
    }

    public async Task<BulkUpdateResult> UpdateSeriesAsync(BulkUpdateRequest request, CancellationToken ct = default)
    {
        ValidateBatchSize(request.Items.Count);
        var result = new BulkUpdateResult();

        foreach (var item in request.Items)
        {
            try
            {
                var series = await _seriesRepository.FindAsync(item.Id, ct).ConfigureAwait(false);
                if (series == null)
                {
                    AddError(result, item.Id, $"Series with ID {item.Id} not found");
                    continue;
                }

                if (item.Monitored.HasValue) series.Monitored = item.Monitored.Value;
                if (item.QualityProfileId.HasValue) series.QualityProfileId = item.QualityProfileId.Value;
                if (item.Path != null) series.Path = item.Path;
                if (item.RootFolderPath != null) series.RootFolderPath = item.RootFolderPath;
                if (item.Tags != null) series.Tags = item.Tags.ToHashSet();

                await _seriesRepository.UpdateAsync(series, ct).ConfigureAwait(false);
                result.UpdatedIds.Add(item.Id);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to update series {Id}", item.Id);
                AddError(result, item.Id, ex.Message);
            }
        }

        result.Updated = result.UpdatedIds.Count;
        return result;
    }

    public async Task<BulkDeleteResult> DeleteSeriesAsync(BulkDeleteRequest request, CancellationToken ct = default)
    {
        ValidateBatchSize(request.Ids.Count);
        var result = new BulkDeleteResult();

        foreach (var id in request.Ids)
        {
            try
            {
                var series = await _seriesRepository.FindAsync(id, ct).ConfigureAwait(false);
                if (series == null)
                {
                    AddError(result, id, $"Series with ID {id} not found");
                    continue;
                }

                await _seriesRepository.DeleteAsync(id, ct).ConfigureAwait(false);
                result.DeletedIds.Add(id);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to delete series {Id}", id);
                AddError(result, id, ex.Message);
            }
        }

        result.Deleted = result.DeletedIds.Count;
        return result;
    }

    public async Task<BulkReadResult> MarkMangaChaptersReadAsync(BulkReadRequest request, CancellationToken ct = default)
    {
        ValidateBatchSize(request.Ids.Count);
        var result = new BulkReadResult();

        foreach (var id in request.Ids)
        {
            try
            {
                if (request.IsRead)
                    await _mangaChapterRepository.MarkReadAsync(id, ct).ConfigureAwait(false);
                else
                    await _mangaChapterRepository.MarkUnreadAsync(id, ct).ConfigureAwait(false);

                result.UpdatedIds.Add(id);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to mark manga chapter {Id} as read", id);
            }
        }

        result.Updated = result.UpdatedIds.Count;
        return result;
    }

    public async Task<BulkReadResult> MarkWebcomicEpisodesReadAsync(BulkReadRequest request, CancellationToken ct = default)
    {
        ValidateBatchSize(request.Ids.Count);
        var result = new BulkReadResult();

        foreach (var id in request.Ids)
        {
            try
            {
                if (request.IsRead)
                    await _webcomicEpisodeRepository.MarkReadAsync(id, ct).ConfigureAwait(false);
                else
                    await _webcomicEpisodeRepository.MarkUnreadAsync(id, ct).ConfigureAwait(false);

                result.UpdatedIds.Add(id);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to mark webcomic episode {Id} as read", id);
            }
        }

        result.Updated = result.UpdatedIds.Count;
        return result;
    }

    public async Task<BulkReadResult> MarkComicIssuesReadAsync(BulkReadRequest request, CancellationToken ct = default)
    {
        ValidateBatchSize(request.Ids.Count);
        var result = new BulkReadResult();

        foreach (var id in request.Ids)
        {
            try
            {
                if (request.IsRead)
                    await _comicIssueRepository.MarkReadAsync(id, ct).ConfigureAwait(false);
                else
                    await _comicIssueRepository.MarkUnreadAsync(id, ct).ConfigureAwait(false);

                result.UpdatedIds.Add(id);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to mark comic issue {Id} as read", id);
            }
        }

        result.Updated = result.UpdatedIds.Count;
        return result;
    }

    public async Task<BulkReadResult> MarkArticlesReadAsync(BulkReadRequest request, CancellationToken ct = default)
    {
        ValidateBatchSize(request.Ids.Count);
        var result = new BulkReadResult();

        foreach (var id in request.Ids)
        {
            try
            {
                if (request.IsRead)
                    await _newsArticleRepository.MarkReadAsync(id, ct).ConfigureAwait(false);
                else
                    await _newsArticleRepository.MarkUnreadAsync(id, ct).ConfigureAwait(false);

                result.UpdatedIds.Add(id);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to mark article {Id} as read", id);
            }
        }

        result.Updated = result.UpdatedIds.Count;
        return result;
    }

    private static void ValidateBatchSize(int count)
    {
        if (count > MaxBatchSize)
        {
            throw new ArgumentException($"Batch size exceeds maximum of {MaxBatchSize} items");
        }
    }

    private static void ApplyCommonUpdates(dynamic entity, BulkUpdateItem item)
    {
        if (item.Monitored.HasValue) entity.Monitored = item.Monitored.Value;
        if (item.QualityProfileId.HasValue) entity.QualityProfileId = item.QualityProfileId.Value;
        if (item.Path != null) entity.Path = item.Path;
        if (item.RootFolderPath != null) entity.RootFolderPath = item.RootFolderPath;
        if (item.Tags != null) entity.Tags = item.Tags;
    }

    private static void AddError(BulkUpdateResult result, int id, string message)
    {
        result.Errors ??= new List<BulkError>();
        result.Errors.Add(new BulkError { Id = id, Message = message });
    }

    private static void AddError(BulkDeleteResult result, int id, string message)
    {
        result.Errors ??= new List<BulkError>();
        result.Errors.Add(new BulkError { Id = id, Message = message });
    }
}

// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Moq;
using Mouseion.Core.Audiobooks;
using Mouseion.Core.Books;
using Mouseion.Core.MediaItems;
using Mouseion.Core.MediaTypes;
using Mouseion.Core.Movies;
using Mouseion.Core.Tags;
using Mouseion.Core.Tags.AutoTagging;

namespace Mouseion.Core.Tests.Tags;

public class AutoTaggingServiceTests
{
    private readonly Mock<IAutoTaggingRuleRepository> _ruleRepository;
    private readonly Mock<ITagRepository> _tagRepository;
    private readonly Mock<IMediaItemRepository> _mediaItemRepository;
    private readonly Mock<ILogger<AutoTaggingService>> _logger;
    private readonly AutoTaggingService _service;

    public AutoTaggingServiceTests()
    {
        _ruleRepository = new Mock<IAutoTaggingRuleRepository>();
        _tagRepository = new Mock<ITagRepository>();
        _mediaItemRepository = new Mock<IMediaItemRepository>();
        _logger = new Mock<ILogger<AutoTaggingService>>();
        _service = new AutoTaggingService(
            _ruleRepository.Object,
            _tagRepository.Object,
            _mediaItemRepository.Object,
            _logger.Object);
    }

    [Fact]
    public async Task EvaluateRulesAsync_NoRules_ReturnsEmpty()
    {
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(new List<AutoTaggingRule>());

        var movie = CreateMovie("Test Movie", new List<string> { "Action" });
        var result = await _service.EvaluateRulesAsync(movie);

        Assert.Empty(result);
    }

    [Fact]
    public async Task EvaluateRulesAsync_GenreContainsRule_MatchesGenre()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "Action Genre",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.GenreContains,
                ConditionValue = "Action",
                TagId = 10
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var movie = CreateMovie("Die Hard", new List<string> { "Action", "Thriller" });
        var result = await _service.EvaluateRulesAsync(movie);

        Assert.Single(result);
        Assert.Contains(10, result);
    }

    [Fact]
    public async Task EvaluateRulesAsync_GenreContainsRule_DoesNotMatchWhenNoGenre()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "Action Genre",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.GenreContains,
                ConditionValue = "Action",
                TagId = 10
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var movie = CreateMovie("The Notebook", new List<string> { "Romance", "Drama" });
        var result = await _service.EvaluateRulesAsync(movie);

        Assert.Empty(result);
    }

    [Fact]
    public async Task EvaluateRulesAsync_MediaTypeFilter_AppliesOnlyToMatchingType()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "Action Movies Only",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.GenreContains,
                ConditionValue = "Action",
                TagId = 10,
                MediaTypeFilter = MediaType.Movie
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var movie = CreateMovie("Die Hard", new List<string> { "Action" });
        var result = await _service.EvaluateRulesAsync(movie);

        Assert.Single(result);
        Assert.Contains(10, result);
    }

    [Fact]
    public async Task EvaluateRulesAsync_MultipleRules_ReturnsAllMatchingTags()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "Action Genre",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.GenreContains,
                ConditionValue = "Action",
                TagId = 10
            },
            new AutoTaggingRule
            {
                Id = 2,
                Name = "Thriller Genre",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.GenreContains,
                ConditionValue = "Thriller",
                TagId = 20
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var movie = CreateMovie("Die Hard", new List<string> { "Action", "Thriller" });
        var result = await _service.EvaluateRulesAsync(movie);

        Assert.Equal(2, result.Count);
        Assert.Contains(10, result);
        Assert.Contains(20, result);
    }

    [Fact]
    public async Task ApplyAutoTagsAsync_AddsMatchingTagsToItem()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "Action Genre",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.GenreContains,
                ConditionValue = "Action",
                TagId = 10
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var movie = CreateMovie("Die Hard", new List<string> { "Action" });
        Assert.Empty(movie.Tags);

        await _service.ApplyAutoTagsAsync(movie);

        Assert.Single(movie.Tags);
        Assert.Contains(10, movie.Tags);
    }

    [Fact]
    public async Task ApplyAutoTagsAsync_DoesNotDuplicateTags()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "Action Genre",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.GenreContains,
                ConditionValue = "Action",
                TagId = 10
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var movie = CreateMovie("Die Hard", new List<string> { "Action" });
        movie.Tags.Add(10);

        await _service.ApplyAutoTagsAsync(movie);

        Assert.Single(movie.Tags);
    }

    [Fact]
    public void EvaluateRules_Sync_MatchesGenre()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "Action Genre",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.GenreContains,
                ConditionValue = "Action",
                TagId = 10
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRules())
            .Returns(rules);

        var movie = CreateMovie("Die Hard", new List<string> { "Action" });
        var result = _service.EvaluateRules(movie);

        Assert.Single(result);
        Assert.Contains(10, result);
    }

    [Fact]
    public void ApplyAutoTags_Sync_AddsMatchingTags()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "Action Genre",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.GenreContains,
                ConditionValue = "Action",
                TagId = 10
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRules())
            .Returns(rules);

        var movie = CreateMovie("Die Hard", new List<string> { "Action" });
        _service.ApplyAutoTags(movie);

        Assert.Single(movie.Tags);
        Assert.Contains(10, movie.Tags);
    }

    [Fact]
    public async Task EvaluateRulesAsync_CaseInsensitiveGenreMatch()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "Action Genre",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.GenreContains,
                ConditionValue = "action",
                TagId = 10
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var movie = CreateMovie("Die Hard", new List<string> { "ACTION" });
        var result = await _service.EvaluateRulesAsync(movie);

        Assert.Single(result);
        Assert.Contains(10, result);
    }

    [Fact]
    public async Task EvaluateRulesAsync_PartialGenreMatch()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "Science Fiction",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.GenreContains,
                ConditionValue = "Sci",
                TagId = 10
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var movie = CreateMovie("Star Wars", new List<string> { "Science Fiction", "Adventure" });
        var result = await _service.EvaluateRulesAsync(movie);

        Assert.Single(result);
        Assert.Contains(10, result);
    }

    [Fact]
    public async Task EvaluateRulesAsync_LanguageContainsRule_MatchesBookLanguage()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "English Books",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.LanguageContains,
                ConditionValue = "eng",
                TagId = 10
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var book = CreateBook("Test Book", "english");
        var result = await _service.EvaluateRulesAsync(book);

        Assert.Single(result);
        Assert.Contains(10, result);
    }

    [Fact]
    public async Task EvaluateRulesAsync_LanguageContainsRule_DoesNotMatchWhenNoLanguage()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "English Books",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.LanguageContains,
                ConditionValue = "eng",
                TagId = 10
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var book = CreateBook("Test Book", null);
        var result = await _service.EvaluateRulesAsync(book);

        Assert.Empty(result);
    }

    [Fact]
    public async Task EvaluateRulesAsync_LanguageContainsRule_MatchesAudiobookLanguage()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "Japanese Audiobooks",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.LanguageContains,
                ConditionValue = "japan",
                TagId = 20
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var audiobook = CreateAudiobook("Test Audiobook", "japanese");
        var result = await _service.EvaluateRulesAsync(audiobook);

        Assert.Single(result);
        Assert.Contains(20, result);
    }

    [Fact]
    public async Task EvaluateRulesAsync_QualityEqualsRule_ReturnsFalse()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "HD Quality",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.QualityEquals,
                ConditionValue = "HD",
                TagId = 10
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var movie = CreateMovie("Test", new List<string>());
        var result = await _service.EvaluateRulesAsync(movie);

        Assert.Empty(result);
    }

    [Fact]
    public async Task EvaluateRulesAsync_FormatEqualsRule_ReturnsFalse()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "FLAC Format",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.FormatEquals,
                ConditionValue = "FLAC",
                TagId = 10
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var movie = CreateMovie("Test", new List<string>());
        var result = await _service.EvaluateRulesAsync(movie);

        Assert.Empty(result);
    }

    [Fact]
    public async Task EvaluateRulesAsync_BitDepthAtLeastRule_ReturnsFalse()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "Hi-Res",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.BitDepthAtLeast,
                ConditionValue = "24",
                TagId = 10
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var movie = CreateMovie("Test", new List<string>());
        var result = await _service.EvaluateRulesAsync(movie);

        Assert.Empty(result);
    }

    [Fact]
    public async Task EvaluateRulesAsync_CustomRule_ReturnsFalse()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "Custom Rule",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.Custom,
                ConditionValue = ".*",
                TagId = 10
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var movie = CreateMovie("Test", new List<string>());
        var result = await _service.EvaluateRulesAsync(movie);

        Assert.Empty(result);
    }

    [Fact]
    public async Task EvaluateRulesAsync_RuleWithWrongMediaType_DoesNotMatch()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "Book Only Rule",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.GenreContains,
                ConditionValue = "Action",
                TagId = 10,
                MediaTypeFilter = MediaType.Book
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var movie = CreateMovie("Die Hard", new List<string> { "Action" });
        var result = await _service.EvaluateRulesAsync(movie);

        Assert.Empty(result);
    }

    [Fact]
    public async Task ApplyAutoTagsAsync_NoMatchingTags_DoesNotModify()
    {
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(new List<AutoTaggingRule>());

        var movie = CreateMovie("Test", new List<string>());
        movie.Tags.Add(5);

        await _service.ApplyAutoTagsAsync(movie);

        Assert.Single(movie.Tags);
        Assert.Contains(5, movie.Tags);
    }

    [Fact]
    public async Task ApplyAutoTagsToAllAsync_NoRules_ReturnsZero()
    {
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(new List<AutoTaggingRule>());

        var result = await _service.ApplyAutoTagsToAllAsync(null);

        Assert.Equal(0, result);
    }

    [Fact]
    public async Task ApplyAutoTagsToAllAsync_WithMatchingItems_ReturnsUpdatedCount()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "Action Genre",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.GenreContains,
                ConditionValue = "Action",
                TagId = 10
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var movie1 = CreateMovie("Die Hard", new List<string> { "Action" });
        var movie2 = CreateMovie("Notebook", new List<string> { "Romance" });

        var summaries = new List<MediaItemSummary>
        {
            new MediaItemSummary { Id = 1 },
            new MediaItemSummary { Id = 2 }
        };
        _mediaItemRepository.Setup(x => x.GetPageAsync(1, int.MaxValue, null, It.IsAny<CancellationToken>()))
            .ReturnsAsync(summaries);
        _mediaItemRepository.Setup(x => x.FindByIdAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(movie1);
        _mediaItemRepository.Setup(x => x.FindByIdAsync(2, It.IsAny<CancellationToken>()))
            .ReturnsAsync(movie2);

        var result = await _service.ApplyAutoTagsToAllAsync(null);

        Assert.Equal(1, result);
        Assert.Contains(10, movie1.Tags);
        Assert.Empty(movie2.Tags);
    }

    [Fact]
    public async Task ApplyAutoTagsToAllAsync_WithMediaTypeFilter_FiltersItems()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "Action Genre",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.GenreContains,
                ConditionValue = "Action",
                TagId = 10
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var movie = CreateMovie("Die Hard", new List<string> { "Action" });
        var summaries = new List<MediaItemSummary> { new MediaItemSummary { Id = 1 } };

        _mediaItemRepository.Setup(x => x.GetPageAsync(1, int.MaxValue, MediaType.Movie, It.IsAny<CancellationToken>()))
            .ReturnsAsync(summaries);
        _mediaItemRepository.Setup(x => x.FindByIdAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(movie);

        var result = await _service.ApplyAutoTagsToAllAsync(MediaType.Movie);

        Assert.Equal(1, result);
    }

    [Fact]
    public async Task ApplyAutoTagsToAllAsync_ItemNotFound_ContinuesProcessing()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "Action Genre",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.GenreContains,
                ConditionValue = "Action",
                TagId = 10
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var summaries = new List<MediaItemSummary>
        {
            new MediaItemSummary { Id = 1 },
            new MediaItemSummary { Id = 2 }
        };
        _mediaItemRepository.Setup(x => x.GetPageAsync(1, int.MaxValue, null, It.IsAny<CancellationToken>()))
            .ReturnsAsync(summaries);
        _mediaItemRepository.Setup(x => x.FindByIdAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync((MediaItem?)null);
        _mediaItemRepository.Setup(x => x.FindByIdAsync(2, It.IsAny<CancellationToken>()))
            .ReturnsAsync(CreateMovie("Die Hard", new List<string> { "Action" }));

        var result = await _service.ApplyAutoTagsToAllAsync(null);

        Assert.Equal(1, result);
    }

    [Fact]
    public async Task PreviewTagsAsync_ReturnsMatchingTags()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "Action Genre",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.GenreContains,
                ConditionValue = "Action",
                TagId = 10
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var movie = CreateMovie("Die Hard", new List<string> { "Action" });
        var result = await _service.PreviewTagsAsync(movie);

        Assert.Single(result);
        Assert.Contains(10, result);
    }

    [Fact]
    public async Task EvaluateRulesAsync_BookGenreContains_MatchesBookGenres()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "Fantasy Books",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.GenreContains,
                ConditionValue = "Fantasy",
                TagId = 10
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var book = CreateBook("Lord of the Rings", null, new List<string> { "Fantasy", "Adventure" });
        var result = await _service.EvaluateRulesAsync(book);

        Assert.Single(result);
        Assert.Contains(10, result);
    }

    [Fact]
    public async Task EvaluateRulesAsync_AudiobookGenreContains_MatchesAudiobookGenres()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "Mystery Audiobooks",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.GenreContains,
                ConditionValue = "Mystery",
                TagId = 15
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var audiobook = CreateAudiobook("Murder Mystery", null, new List<string> { "Mystery", "Thriller" });
        var result = await _service.EvaluateRulesAsync(audiobook);

        Assert.Single(result);
        Assert.Contains(15, result);
    }

    [Fact]
    public async Task EvaluateRulesAsync_ItemWithExistingTag_DoesNotDuplicateOnEval()
    {
        var rules = new List<AutoTaggingRule>
        {
            new AutoTaggingRule
            {
                Id = 1,
                Name = "Action Genre",
                Enabled = true,
                ConditionType = AutoTaggingConditionType.GenreContains,
                ConditionValue = "Action",
                TagId = 10
            }
        };
        _ruleRepository.Setup(x => x.GetEnabledRulesAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var movie = CreateMovie("Die Hard", new List<string> { "Action" });
        movie.Tags.Add(10);

        var result = await _service.EvaluateRulesAsync(movie);

        Assert.Single(result);
    }

    private static Movie CreateMovie(string title, List<string> genres)
    {
        return new Movie
        {
            Id = 1,
            Title = title,
            Year = 2024,
            Genres = genres,
            QualityProfileId = 1
        };
    }

    private static Book CreateBook(string title, string? language, List<string>? genres = null)
    {
        return new Book
        {
            Id = 1,
            Title = title,
            QualityProfileId = 1,
            Metadata = new BookMetadata
            {
                Language = language ?? string.Empty,
                Genres = genres ?? new List<string>()
            }
        };
    }

    private static Audiobook CreateAudiobook(string title, string? language, List<string>? genres = null)
    {
        return new Audiobook
        {
            Id = 1,
            Title = title,
            QualityProfileId = 1,
            Metadata = new AudiobookMetadata
            {
                Language = language ?? string.Empty,
                Genres = genres ?? new List<string>()
            }
        };
    }
}

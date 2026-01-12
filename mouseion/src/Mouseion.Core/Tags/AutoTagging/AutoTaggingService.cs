// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.Books;
using Mouseion.Core.Audiobooks;
using Mouseion.Core.MediaItems;
using Mouseion.Core.MediaTypes;
using Mouseion.Core.Movies;
using Mouseion.Core.Music;

namespace Mouseion.Core.Tags.AutoTagging;

public class AutoTaggingService : IAutoTaggingService
{
    private readonly IAutoTaggingRuleRepository _ruleRepository;
    private readonly ITagRepository _tagRepository;
    private readonly IMediaItemRepository _mediaItemRepository;
    private readonly ILogger<AutoTaggingService> _logger;

    private List<AutoTaggingRule>? _cachedRules;
    private DateTime _cacheExpiry = DateTime.MinValue;
    private static readonly TimeSpan CacheDuration = TimeSpan.FromMinutes(5);

    public AutoTaggingService(
        IAutoTaggingRuleRepository ruleRepository,
        ITagRepository tagRepository,
        IMediaItemRepository mediaItemRepository,
        ILogger<AutoTaggingService> logger)
    {
        _ruleRepository = ruleRepository;
        _tagRepository = tagRepository;
        _mediaItemRepository = mediaItemRepository;
        _logger = logger;
    }

    public async Task<HashSet<int>> EvaluateRulesAsync(MediaItem item, CancellationToken ct = default)
    {
        var rules = await GetEnabledRulesAsync(ct).ConfigureAwait(false);
        return EvaluateRulesInternal(item, rules);
    }

    public HashSet<int> EvaluateRules(MediaItem item)
    {
        var rules = GetEnabledRules();
        return EvaluateRulesInternal(item, rules);
    }

    public async Task ApplyAutoTagsAsync(MediaItem item, CancellationToken ct = default)
    {
        var matchingTags = await EvaluateRulesAsync(item, ct).ConfigureAwait(false);
        if (matchingTags.Count == 0)
        {
            return;
        }

        var beforeCount = item.Tags.Count;
        item.Tags.UnionWith(matchingTags);

        if (item.Tags.Count > beforeCount)
        {
            _logger.LogDebug("Applied {Count} auto-tags to {Title}", item.Tags.Count - beforeCount, item.GetTitle());
        }
    }

    public void ApplyAutoTags(MediaItem item)
    {
        var matchingTags = EvaluateRules(item);
        if (matchingTags.Count == 0)
        {
            return;
        }

        var beforeCount = item.Tags.Count;
        item.Tags.UnionWith(matchingTags);

        if (item.Tags.Count > beforeCount)
        {
            _logger.LogDebug("Applied {Count} auto-tags to {Title}", item.Tags.Count - beforeCount, item.GetTitle());
        }
    }

    public async Task<int> ApplyAutoTagsToAllAsync(MediaType? mediaType, CancellationToken ct = default)
    {
        var rules = await GetEnabledRulesAsync(ct).ConfigureAwait(false);
        if (rules.Count == 0)
        {
            return 0;
        }

        var items = await _mediaItemRepository.GetPageAsync(1, int.MaxValue, mediaType, ct).ConfigureAwait(false);
        var updatedCount = 0;

        foreach (var summary in items)
        {
            var item = await _mediaItemRepository.FindByIdAsync(summary.Id, ct).ConfigureAwait(false);
            if (item == null) continue;

            var matchingTags = EvaluateRulesInternal(item, rules);
            if (matchingTags.Count == 0) continue;

            var beforeCount = item.Tags.Count;
            item.Tags.UnionWith(matchingTags);

            if (item.Tags.Count > beforeCount)
            {
                updatedCount++;
            }
        }

        _logger.LogInformation("Applied auto-tags to {Count} items", updatedCount);
        return updatedCount;
    }

    public async Task<HashSet<int>> PreviewTagsAsync(MediaItem item, CancellationToken ct = default)
    {
        return await EvaluateRulesAsync(item, ct).ConfigureAwait(false);
    }

    private HashSet<int> EvaluateRulesInternal(MediaItem item, List<AutoTaggingRule> rules)
    {
        var matchingTags = new HashSet<int>();
        var applicableRules = rules.Where(r => r.MediaTypeFilter == null || r.MediaTypeFilter == item.MediaType);

        foreach (var rule in applicableRules)
        {
            if (EvaluateCondition(item, rule))
            {
                matchingTags.Add(rule.TagId);
            }
        }

        return matchingTags;
    }

    private bool EvaluateCondition(MediaItem item, AutoTaggingRule rule)
    {
        return rule.ConditionType switch
        {
            AutoTaggingConditionType.GenreContains => EvaluateGenreContains(item, rule.ConditionValue),
            AutoTaggingConditionType.LanguageContains => EvaluateLanguageContains(item, rule.ConditionValue),
            AutoTaggingConditionType.QualityEquals => EvaluateQualityEquals(item, rule.ConditionValue),
            AutoTaggingConditionType.QualityGroupEquals => EvaluateQualityGroupEquals(item, rule.ConditionValue),
            AutoTaggingConditionType.FormatEquals => EvaluateFormatEquals(item, rule.ConditionValue),
            AutoTaggingConditionType.BitDepthAtLeast => EvaluateBitDepthAtLeast(item, rule.ConditionValue),
            AutoTaggingConditionType.Custom => false,
            _ => false
        };
    }

    private static bool EvaluateGenreContains(MediaItem item, string value)
    {
        var genres = GetGenres(item);
        return genres.Any(g => g.Contains(value, StringComparison.OrdinalIgnoreCase));
    }

    private static bool EvaluateLanguageContains(MediaItem item, string value)
    {
        if (item is Book book && !string.IsNullOrEmpty(book.Metadata.Language))
        {
            return book.Metadata.Language.Contains(value, StringComparison.OrdinalIgnoreCase);
        }

        if (item is Audiobook audiobook && !string.IsNullOrEmpty(audiobook.Metadata.Language))
        {
            return audiobook.Metadata.Language.Contains(value, StringComparison.OrdinalIgnoreCase);
        }

        return false;
    }

    private static bool EvaluateQualityEquals(MediaItem item, string value)
    {
        return false;
    }

    private static bool EvaluateQualityGroupEquals(MediaItem item, string value)
    {
        return false;
    }

    private static bool EvaluateFormatEquals(MediaItem item, string value)
    {
        return false;
    }

    private static bool EvaluateBitDepthAtLeast(MediaItem item, string value)
    {
        return false;
    }

    private static List<string> GetGenres(MediaItem item)
    {
        return item switch
        {
            Movie movie => movie.Genres,
            Book book => book.Metadata.Genres,
            Audiobook audiobook => audiobook.Metadata.Genres,
            _ => new List<string>()
        };
    }

    private async Task<List<AutoTaggingRule>> GetEnabledRulesAsync(CancellationToken ct)
    {
        if (_cachedRules != null && DateTime.UtcNow < _cacheExpiry)
        {
            return _cachedRules;
        }

        _cachedRules = await _ruleRepository.GetEnabledRulesAsync(ct).ConfigureAwait(false);
        _cacheExpiry = DateTime.UtcNow.Add(CacheDuration);
        return _cachedRules;
    }

    private List<AutoTaggingRule> GetEnabledRules()
    {
        if (_cachedRules != null && DateTime.UtcNow < _cacheExpiry)
        {
            return _cachedRules;
        }

        _cachedRules = _ruleRepository.GetEnabledRules();
        _cacheExpiry = DateTime.UtcNow.Add(CacheDuration);
        return _cachedRules;
    }
}

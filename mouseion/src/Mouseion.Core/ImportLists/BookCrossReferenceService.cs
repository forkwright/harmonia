// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;

namespace Mouseion.Core.ImportLists;

/// <summary>
/// Cross-references book imports from multiple sources (Goodreads, OpenLibrary, etc.)
/// to avoid duplicates when the same user imports from both services.
/// Matching priority: ISBN-13 > ISBN-10 > title+author fuzzy match.
/// </summary>
public interface IBookCrossReferenceService
{
    /// <summary>
    /// Deduplicate a combined list of book import items from multiple sources.
    /// Prefers items with more metadata (ratings, read dates, etc.).
    /// </summary>
    List<ImportListItem> DeduplicateAcrossSources(List<ImportListItem> items);
}

public class BookCrossReferenceService : IBookCrossReferenceService
{
    private readonly ILogger<BookCrossReferenceService> _logger;

    public BookCrossReferenceService(ILogger<BookCrossReferenceService> logger)
    {
        _logger = logger;
    }

    public List<ImportListItem> DeduplicateAcrossSources(List<ImportListItem> items)
    {
        var bookItems = items
            .Where(i => i.MediaType == MediaTypes.MediaType.Book || i.MediaType == MediaTypes.MediaType.Audiobook)
            .ToList();

        var nonBookItems = items
            .Where(i => i.MediaType != MediaTypes.MediaType.Book && i.MediaType != MediaTypes.MediaType.Audiobook)
            .ToList();

        if (bookItems.Count == 0)
            return items;

        var deduplicated = new List<ImportListItem>();
        var isbnGroups = new Dictionary<string, List<ImportListItem>>();

        // Group by normalized ISBN
        foreach (var item in bookItems)
        {
            var isbn = NormalizeIsbn(item.Isbn);
            if (!string.IsNullOrEmpty(isbn))
            {
                if (!isbnGroups.ContainsKey(isbn))
                    isbnGroups[isbn] = new List<ImportListItem>();
                isbnGroups[isbn].Add(item);
            }
            else
            {
                // No ISBN — can't cross-reference, keep as-is
                deduplicated.Add(item);
            }
        }

        // For each ISBN group, pick the best item
        foreach (var (isbn, group) in isbnGroups)
        {
            if (group.Count == 1)
            {
                deduplicated.Add(group[0]);
                continue;
            }

            var best = PickBestItem(group);
            deduplicated.Add(best);

            _logger.LogDebug(
                "Cross-reference: merged {Count} sources for ISBN {Isbn} ({Title})",
                group.Count, isbn, best.Title);
        }

        // Title+author fuzzy match for items without ISBN that might still be duplicates
        var noIsbnItems = deduplicated.Where(i => string.IsNullOrEmpty(NormalizeIsbn(i.Isbn))).ToList();
        var withIsbnItems = deduplicated.Where(i => !string.IsNullOrEmpty(NormalizeIsbn(i.Isbn))).ToList();

        var finalNoIsbn = new List<ImportListItem>();
        foreach (var item in noIsbnItems)
        {
            var existingMatch = withIsbnItems.FirstOrDefault(existing =>
                FuzzyTitleAuthorMatch(existing, item));

            if (existingMatch != null)
            {
                // This item is a duplicate of an ISBN-matched item — skip it
                _logger.LogDebug(
                    "Cross-reference: dropped duplicate (title match) for {Title} by {Author}",
                    item.Title, item.Author);
                continue;
            }

            finalNoIsbn.Add(item);
        }

        nonBookItems.AddRange(withIsbnItems);
        nonBookItems.AddRange(finalNoIsbn);
        return nonBookItems;
    }

    private static ImportListItem PickBestItem(List<ImportListItem> group)
    {
        // Score each item by metadata completeness
        return group.OrderByDescending(MetadataScore).First();
    }

    private static int MetadataScore(ImportListItem item)
    {
        var score = 0;
        if (item.GoodreadsId > 0) score += 2; // Goodreads has richer metadata
        if (!string.IsNullOrEmpty(item.Isbn)) score += 1;
        if (item.UserRating.HasValue && item.UserRating.Value > 0) score += 2;
        if (item.WatchedAt.HasValue) score += 1;
        if (!string.IsNullOrEmpty(item.Author)) score += 1;
        if (item.Year > 0) score += 1;
        return score;
    }

    private static bool FuzzyTitleAuthorMatch(ImportListItem a, ImportListItem b)
    {
        var titleMatch = NormalizeForMatch(a.Title) == NormalizeForMatch(b.Title);
        var authorMatch = NormalizeForMatch(a.Author ?? "") == NormalizeForMatch(b.Author ?? "");

        return titleMatch && authorMatch;
    }

    private static string NormalizeForMatch(string value)
    {
        // Lowercase, remove punctuation and extra whitespace
        return new string(value.ToLowerInvariant()
            .Where(c => char.IsLetterOrDigit(c) || c == ' ')
            .ToArray())
            .Trim();
    }

    private static string? NormalizeIsbn(string? isbn)
    {
        if (string.IsNullOrWhiteSpace(isbn))
            return null;

        // Strip hyphens and spaces, keep only digits and X (for ISBN-10 check digit)
        var normalized = new string(isbn.Where(c => char.IsDigit(c) || c == 'X' || c == 'x').ToArray());

        // Valid ISBNs are 10 or 13 digits
        if (normalized.Length == 10 || normalized.Length == 13)
            return normalized;

        return null;
    }
}

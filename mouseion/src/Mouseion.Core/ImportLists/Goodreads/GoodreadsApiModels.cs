// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.ImportLists.Goodreads;

/// <summary>
/// Represents a book item parsed from a Goodreads RSS shelf feed.
/// RSS structure: channel > item with title, author_name, isbn, book_id, etc.
/// Goodreads deprecated their API in 2020 but RSS feeds remain functional.
/// Feed URL: https://www.goodreads.com/review/list_rss/{userId}?shelf={shelfName}
/// </summary>
public class GoodreadsRssItem
{
    public string Title { get; set; } = string.Empty;
    public string AuthorName { get; set; } = string.Empty;
    public long BookId { get; set; }
    public string? Isbn { get; set; }
    public string? Isbn13 { get; set; }
    public int? UserRating { get; set; }
    public int? AverageRating { get; set; }
    public string? BookPublished { get; set; }
    public string? ImageUrl { get; set; }

    /// <summary>
    /// Format/binding from the edition (e.g., "Paperback", "Hardcover", "Audio CD", "Kindle Edition", "Audiobook")
    /// Used for audiobook detection.
    /// </summary>
    public string? Format { get; set; }

    /// <summary>
    /// Number of pages. Null for audiobooks.
    /// </summary>
    public int? NumPages { get; set; }

    /// <summary>
    /// Date the user marked this book as read (from RSS "user_read_at" field).
    /// </summary>
    public DateTime? UserReadAt { get; set; }

    /// <summary>
    /// Date the user added this book to the shelf.
    /// </summary>
    public DateTime? UserDateAdded { get; set; }

    /// <summary>
    /// User's text review, if any.
    /// </summary>
    public string? UserReview { get; set; }
}

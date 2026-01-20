// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.News;

public class NewsArticle : ModelBase
{
    public int NewsFeedId { get; set; }
    public string Title { get; set; } = string.Empty;
    public string? Author { get; set; }
    public string? ArticleGuid { get; set; }
    public string? SourceUrl { get; set; }
    public DateTime? PublishDate { get; set; }
    public string? Description { get; set; }
    public string? Content { get; set; }
    public string? ContentHash { get; set; }
    public string? ImageUrl { get; set; }
    public string? Categories { get; set; }
    public bool IsRead { get; set; }
    public bool IsStarred { get; set; }
    public bool IsArchived { get; set; }
    public string? ArchivedContent { get; set; }
    public DateTime Added { get; set; }

    public override string ToString() => Title;
}

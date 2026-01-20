// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(22, "Add News feed and article tables")]
public class Migration022AddNewsTables : FluentMigrator.Migration
{
    public override void Up()
    {
        Create.Table("NewsFeeds")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("Title").AsString().NotNullable()
            .WithColumn("SortTitle").AsString().Nullable()
            .WithColumn("Description").AsString().Nullable()
            .WithColumn("FeedUrl").AsString().NotNullable().Unique()
            .WithColumn("SiteUrl").AsString().Nullable()
            .WithColumn("FaviconUrl").AsString().Nullable()
            .WithColumn("Category").AsString().Nullable()
            .WithColumn("Language").AsString().Nullable()
            .WithColumn("LastFetchTime").AsDateTime().Nullable()
            .WithColumn("LastItemDate").AsDateTime().Nullable()
            .WithColumn("ItemCount").AsInt32().Nullable()
            .WithColumn("Monitored").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("RefreshInterval").AsInt32().NotNullable().WithDefaultValue(60)
            .WithColumn("Path").AsString().Nullable()
            .WithColumn("RootFolderPath").AsString().Nullable()
            .WithColumn("QualityProfileId").AsInt32().NotNullable().WithDefaultValue(1)
            .WithColumn("Tags").AsString().Nullable()
            .WithColumn("Added").AsDateTime().NotNullable();

        Create.Table("NewsArticles")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("NewsFeedId").AsInt32().NotNullable()
            .WithColumn("Title").AsString().NotNullable()
            .WithColumn("Author").AsString().Nullable()
            .WithColumn("ArticleGuid").AsString().Nullable()
            .WithColumn("SourceUrl").AsString().Nullable()
            .WithColumn("PublishDate").AsDateTime().Nullable()
            .WithColumn("Description").AsString().Nullable()
            .WithColumn("Content").AsString().Nullable()
            .WithColumn("ContentHash").AsString().Nullable()
            .WithColumn("ImageUrl").AsString().Nullable()
            .WithColumn("Categories").AsString().Nullable()
            .WithColumn("IsRead").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("IsStarred").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("IsArchived").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("ArchivedContent").AsString().Nullable()
            .WithColumn("Added").AsDateTime().NotNullable();

        Create.Index("IX_NewsFeeds_Monitored")
            .OnTable("NewsFeeds")
            .OnColumn("Monitored");

        // FeedUrl index not needed - Unique() constraint already creates one

        Create.Index("IX_NewsArticles_NewsFeedId")
            .OnTable("NewsArticles")
            .OnColumn("NewsFeedId");

        Create.Index("IX_NewsArticles_ArticleGuid")
            .OnTable("NewsArticles")
            .OnColumn("ArticleGuid");

        Create.Index("IX_NewsArticles_IsRead")
            .OnTable("NewsArticles")
            .OnColumn("IsRead");

        Create.Index("IX_NewsArticles_IsStarred")
            .OnTable("NewsArticles")
            .OnColumn("IsStarred");

        Create.Index("IX_NewsArticles_PublishDate")
            .OnTable("NewsArticles")
            .OnColumn("PublishDate");

        // Note: Foreign key constraint omitted for SQLite compatibility
        // Cascade delete handled at application level
    }

    public override void Down()
    {
        Delete.Table("NewsArticles");
        Delete.Table("NewsFeeds");
    }
}

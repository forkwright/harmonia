// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(3, "Add MediaItems table")]
public class Migration_003_AddMediaItems : FluentMigrator.Migration
{
    public override void Up()
    {
        Create.Table("MediaItems")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("MediaType").AsInt32().NotNullable()
            .WithColumn("Title").AsString().NotNullable()
            .WithColumn("Year").AsInt32().Nullable()
            .WithColumn("Monitored").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("QualityProfileId").AsInt32().NotNullable()
            .WithColumn("Path").AsString().Nullable()
            .WithColumn("RootFolderPath").AsString().Nullable()
            .WithColumn("Added").AsDateTime().NotNullable().WithDefault(SystemMethods.CurrentDateTime)
            .WithColumn("Tags").AsString().Nullable() // JSON array of tag IDs
            .WithColumn("LastSearchTime").AsDateTime().Nullable()
            // Polymorphic metadata JSON columns per media type
            .WithColumn("MovieMetadata").AsString().Nullable()
            .WithColumn("TVMetadata").AsString().Nullable()
            .WithColumn("MusicMetadata").AsString().Nullable()
            .WithColumn("BookMetadata").AsString().Nullable()
            .WithColumn("AudiobookMetadata").AsString().Nullable()
            .WithColumn("PodcastMetadata").AsString().Nullable()
            .WithColumn("ComicMetadata").AsString().Nullable()
            // Foreign keys for hierarchical parent entities
            .WithColumn("AuthorId").AsInt32().Nullable()      // For Books/Audiobooks
            .WithColumn("ArtistId").AsInt32().Nullable()      // For Music (Albums/Tracks)
            .WithColumn("TVShowId").AsInt32().Nullable()      // For TV Episodes
            .WithColumn("BookSeriesId").AsInt32().Nullable(); // For Books in a series

        // Indexes for common queries
        Create.Index("IX_MediaItems_MediaType")
            .OnTable("MediaItems")
            .OnColumn("MediaType");

        Create.Index("IX_MediaItems_Monitored")
            .OnTable("MediaItems")
            .OnColumn("Monitored");

        Create.Index("IX_MediaItems_AuthorId")
            .OnTable("MediaItems")
            .OnColumn("AuthorId");

        Create.Index("IX_MediaItems_ArtistId")
            .OnTable("MediaItems")
            .OnColumn("ArtistId");

        Create.Index("IX_MediaItems_TVShowId")
            .OnTable("MediaItems")
            .OnColumn("TVShowId");

        Create.Index("IX_MediaItems_BookSeriesId")
            .OnTable("MediaItems")
            .OnColumn("BookSeriesId");

        Create.Index("IX_MediaItems_QualityProfileId")
            .OnTable("MediaItems")
            .OnColumn("QualityProfileId");
    }

    public override void Down()
    {
        Delete.Table("MediaItems");
    }
}

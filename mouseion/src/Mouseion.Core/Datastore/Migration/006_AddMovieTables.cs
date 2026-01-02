// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(6, "Add Movie tables (Collections, movie fields, MovieFiles)")]
public class Migration_006_AddMovieTables : FluentMigrator.Migration
{
    public override void Up()
    {
        // Collections table for movie collections (e.g., "Lord of the Rings Trilogy")
        Create.Table("Collections")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("Title").AsString().NotNullable()
            .WithColumn("TmdbId").AsString().Nullable()
            .WithColumn("Overview").AsString().Nullable()
            .WithColumn("Images").AsString().Nullable()  // JSON array
            .WithColumn("Monitored").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("QualityProfileId").AsInt32().NotNullable()
            .WithColumn("Added").AsDateTime().NotNullable();

        // Add Movie columns to MediaItems table (Title and Year already exist from Migration 003)
        Alter.Table("MediaItems")
            .AddColumn("Overview").AsString().Nullable()
            .AddColumn("Runtime").AsInt32().Nullable()
            .AddColumn("TmdbId").AsString().Nullable()
            .AddColumn("ImdbId").AsString().Nullable()
            .AddColumn("Images").AsString().Nullable()  // JSON array
            .AddColumn("Genres").AsString().Nullable()  // JSON array
            .AddColumn("InCinemas").AsDateTime().Nullable()
            .AddColumn("PhysicalRelease").AsDateTime().Nullable()
            .AddColumn("DigitalRelease").AsDateTime().Nullable()
            .AddColumn("Certification").AsString().Nullable()
            .AddColumn("Studio").AsString().Nullable()
            .AddColumn("Website").AsString().Nullable()
            .AddColumn("YouTubeTrailerId").AsString().Nullable()
            .AddColumn("Popularity").AsFloat().Nullable()
            .AddColumn("CollectionId").AsInt32().Nullable();

        // MovieFiles table for physical movie files
        Create.Table("MovieFiles")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("MovieId").AsInt32().Nullable()
            .WithColumn("Path").AsString().NotNullable()
            .WithColumn("Size").AsInt64().NotNullable()
            .WithColumn("DateAdded").AsDateTime().NotNullable()
            .WithColumn("SceneName").AsString().Nullable()
            .WithColumn("ReleaseGroup").AsString().Nullable()
            .WithColumn("Edition").AsString().Nullable()
            .WithColumn("VideoCodec").AsString().Nullable()
            .WithColumn("AudioCodec").AsString().Nullable()
            .WithColumn("Width").AsInt32().Nullable()
            .WithColumn("Height").AsInt32().Nullable()
            .WithColumn("Bitrate").AsInt32().Nullable();
    }

    public override void Down()
    {
        // Drop MovieFiles table
        Delete.Table("MovieFiles");

        // Remove Movie columns from MediaItems (Title and Year are from Migration 003, don't delete)
        Delete.Column("Overview").FromTable("MediaItems");
        Delete.Column("Runtime").FromTable("MediaItems");
        Delete.Column("TmdbId").FromTable("MediaItems");
        Delete.Column("ImdbId").FromTable("MediaItems");
        Delete.Column("Images").FromTable("MediaItems");
        Delete.Column("Genres").FromTable("MediaItems");
        Delete.Column("InCinemas").FromTable("MediaItems");
        Delete.Column("PhysicalRelease").FromTable("MediaItems");
        Delete.Column("DigitalRelease").FromTable("MediaItems");
        Delete.Column("Certification").FromTable("MediaItems");
        Delete.Column("Studio").FromTable("MediaItems");
        Delete.Column("Website").FromTable("MediaItems");
        Delete.Column("YouTubeTrailerId").FromTable("MediaItems");
        Delete.Column("Popularity").FromTable("MediaItems");
        Delete.Column("CollectionId").FromTable("MediaItems");

        // Drop Collections table
        Delete.Table("Collections");
    }
}

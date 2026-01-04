// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(11, "Enhance Tracks for Phase 2 advanced filtering")]
public class Migration011EnhanceTracksForPhase2 : FluentMigrator.Migration
{
    private const string TableMusicFiles = "MusicFiles";
    private const string TableMediaItems = "MediaItems";

    public override void Up()
    {
        // Add audio quality fields to MusicFiles table
        Alter.Table(TableMusicFiles)
            .AddColumn("BitDepth").AsInt32().Nullable()
            .AddColumn("DynamicRange").AsInt32().Nullable()
            .AddColumn("Lossless").AsBoolean().NotNullable().WithDefaultValue(false)
            .AddColumn("Codec").AsString().Nullable();

        // Add denormalized search fields to MediaItems table for Tracks
        Alter.Table(TableMediaItems)
            .AddColumn("ArtistName").AsString().Nullable()
            .AddColumn("AlbumName").AsString().Nullable()
            .AddColumn("Genre").AsString().Nullable();

        // Create indexes for audio quality filtering on MusicFiles
        Create.Index("IX_MusicFiles_Codec")
            .OnTable(TableMusicFiles)
            .OnColumn("Codec");

        Create.Index("IX_MusicFiles_DynamicRange")
            .OnTable(TableMusicFiles)
            .OnColumn("DynamicRange");

        Create.Index("IX_MusicFiles_Lossless")
            .OnTable(TableMusicFiles)
            .OnColumn("Lossless");

        Create.Index("IX_MusicFiles_SampleRate")
            .OnTable(TableMusicFiles)
            .OnColumn("SampleRate");

        Create.Index("IX_MusicFiles_BitDepth")
            .OnTable(TableMusicFiles)
            .OnColumn("BitDepth");

        Create.Index("IX_MusicFiles_AudioFormat")
            .OnTable(TableMusicFiles)
            .OnColumn("AudioFormat");

        // Create indexes for denormalized search fields on MediaItems
        Create.Index("IX_MediaItems_Genre")
            .OnTable(TableMediaItems)
            .OnColumn("Genre");

        Create.Index("IX_MediaItems_ArtistName")
            .OnTable(TableMediaItems)
            .OnColumn("ArtistName");

        Create.Index("IX_MediaItems_AlbumName")
            .OnTable(TableMediaItems)
            .OnColumn("AlbumName");
    }

    public override void Down()
    {
        // Remove indexes from MediaItems
        Delete.Index("IX_MediaItems_AlbumName").OnTable(TableMediaItems);
        Delete.Index("IX_MediaItems_ArtistName").OnTable(TableMediaItems);
        Delete.Index("IX_MediaItems_Genre").OnTable(TableMediaItems);

        // Remove indexes from MusicFiles
        Delete.Index("IX_MusicFiles_AudioFormat").OnTable(TableMusicFiles);
        Delete.Index("IX_MusicFiles_BitDepth").OnTable(TableMusicFiles);
        Delete.Index("IX_MusicFiles_SampleRate").OnTable(TableMusicFiles);
        Delete.Index("IX_MusicFiles_Lossless").OnTable(TableMusicFiles);
        Delete.Index("IX_MusicFiles_DynamicRange").OnTable(TableMusicFiles);
        Delete.Index("IX_MusicFiles_Codec").OnTable(TableMusicFiles);

        // Remove columns from MediaItems
        Delete.Column("Genre").FromTable(TableMediaItems);
        Delete.Column("AlbumName").FromTable(TableMediaItems);
        Delete.Column("ArtistName").FromTable(TableMediaItems);

        // Remove columns from MusicFiles
        Delete.Column("Codec").FromTable(TableMusicFiles);
        Delete.Column("Lossless").FromTable(TableMusicFiles);
        Delete.Column("DynamicRange").FromTable(TableMusicFiles);
        Delete.Column("BitDepth").FromTable(TableMusicFiles);
    }
}

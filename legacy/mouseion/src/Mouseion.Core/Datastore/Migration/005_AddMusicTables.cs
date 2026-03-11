// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(5, "Add Music tables (Albums, track fields, MusicFiles)")]
public class Migration_005_AddMusicTables : FluentMigrator.Migration
{
    public override void Up()
    {
        // Albums table - collection-level metadata for Music
        Create.Table("Albums")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("ArtistId").AsInt32().Nullable()
            .WithColumn("Title").AsString().NotNullable()
            .WithColumn("SortTitle").AsString().Nullable()
            .WithColumn("Description").AsString().Nullable()
            .WithColumn("ForeignAlbumId").AsString().Nullable()  // MusicBrainz Release ID
            .WithColumn("DiscogsId").AsString().Nullable()
            .WithColumn("MusicBrainzId").AsString().Nullable()
            .WithColumn("MediaType").AsInt32().NotNullable()     // Always Music = 6
            .WithColumn("ReleaseDate").AsDateTime().Nullable()
            .WithColumn("AlbumType").AsString().Nullable()       // Album, EP, Single, Live, etc.
            .WithColumn("Images").AsString().Nullable()          // JSON array of image URLs
            .WithColumn("Rating").AsDecimal().Nullable()
            .WithColumn("Votes").AsInt32().Nullable()
            .WithColumn("Genres").AsString().Nullable()          // JSON array
            .WithColumn("TrackCount").AsInt32().Nullable()
            .WithColumn("DiscCount").AsInt32().Nullable()
            .WithColumn("Duration").AsInt32().Nullable()         // Total duration in seconds
            .WithColumn("Monitored").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("QualityProfileId").AsInt32().NotNullable().WithDefaultValue(1)
            .WithColumn("Path").AsString().Nullable()
            .WithColumn("RootFolderPath").AsString().Nullable()
            .WithColumn("Added").AsDateTime().NotNullable().WithDefault(SystemMethods.CurrentDateTime)
            .WithColumn("Tags").AsString().Nullable()            // JSON array of tag IDs
            .WithColumn("LastSearchTime").AsDateTime().Nullable();

        Create.Index("IX_Albums_ArtistId")
            .OnTable("Albums")
            .OnColumn("ArtistId");

        Create.Index("IX_Albums_ForeignAlbumId")
            .OnTable("Albums")
            .OnColumn("ForeignAlbumId");

        Create.Index("IX_Albums_Monitored")
            .OnTable("Albums")
            .OnColumn("Monitored");

        Create.Index("IX_Albums_MediaType")
            .OnTable("Albums")
            .OnColumn("MediaType");

        // Add Track-specific columns to MediaItems table
        Alter.Table("MediaItems")
            .AddColumn("AlbumId").AsInt32().Nullable()
            .AddColumn("ForeignTrackId").AsString().Nullable()  // MusicBrainz Recording ID
            .AddColumn("MusicBrainzId").AsString().Nullable()
            .AddColumn("TrackNumber").AsInt32().Nullable()
            .AddColumn("DiscNumber").AsInt32().Nullable().WithDefaultValue(1)
            .AddColumn("DurationSeconds").AsInt32().Nullable()
            .AddColumn("Explicit").AsBoolean().Nullable().WithDefaultValue(false);

        Create.Index("IX_MediaItems_AlbumId")
            .OnTable("MediaItems")
            .OnColumn("AlbumId");

        Create.Index("IX_MediaItems_ForeignTrackId")
            .OnTable("MediaItems")
            .OnColumn("ForeignTrackId");

        // MusicFiles table - physical file metadata for Tracks
        Create.Table("MusicFiles")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("TrackId").AsInt32().Nullable()
            .WithColumn("AlbumId").AsInt32().Nullable()
            .WithColumn("RelativePath").AsString().Nullable()
            .WithColumn("Size").AsInt64().NotNullable().WithDefaultValue(0)
            .WithColumn("DateAdded").AsDateTime().NotNullable().WithDefault(SystemMethods.CurrentDateTime)
            .WithColumn("SceneName").AsString().Nullable()
            .WithColumn("ReleaseGroup").AsString().Nullable()
            .WithColumn("Quality").AsString().Nullable()         // JSON QualityModel
            .WithColumn("AudioFormat").AsString().Nullable()     // FLAC, MP3, AAC, etc.
            .WithColumn("Bitrate").AsInt32().Nullable()          // kbps
            .WithColumn("SampleRate").AsInt32().Nullable()       // Hz (44100, 48000, 96000, etc.)
            .WithColumn("Channels").AsInt32().Nullable();        // 2 (stereo), 6 (5.1), etc.

        Create.Index("IX_MusicFiles_TrackId")
            .OnTable("MusicFiles")
            .OnColumn("TrackId");

        Create.Index("IX_MusicFiles_AlbumId")
            .OnTable("MusicFiles")
            .OnColumn("AlbumId");

        Create.Index("IX_MusicFiles_RelativePath")
            .OnTable("MusicFiles")
            .OnColumn("RelativePath");

        // Add missing columns to Artists table (migration 004 created Artists but missed some fields)
        Alter.Table("Artists")
            .AddColumn("MusicBrainzId").AsString().Nullable()
            .AddColumn("SpotifyId").AsString().Nullable()
            .AddColumn("LastFmId").AsString().Nullable()
            .AddColumn("Images").AsString().Nullable()           // JSON array of image URLs
            .AddColumn("Rating").AsDecimal().Nullable()
            .AddColumn("Votes").AsInt32().Nullable()
            .AddColumn("Genres").AsString().Nullable()           // JSON array
            .AddColumn("Country").AsString().Nullable()
            .AddColumn("BeginDate").AsDateTime().Nullable()
            .AddColumn("EndDate").AsDateTime().Nullable();

        Create.Index("IX_Artists_MusicBrainzId")
            .OnTable("Artists")
            .OnColumn("MusicBrainzId");
    }

    public override void Down()
    {
        Delete.Index("IX_Artists_MusicBrainzId").OnTable("Artists");
        Delete.Column("EndDate").FromTable("Artists");
        Delete.Column("BeginDate").FromTable("Artists");
        Delete.Column("Country").FromTable("Artists");
        Delete.Column("Genres").FromTable("Artists");
        Delete.Column("Votes").FromTable("Artists");
        Delete.Column("Rating").FromTable("Artists");
        Delete.Column("Images").FromTable("Artists");
        Delete.Column("LastFmId").FromTable("Artists");
        Delete.Column("SpotifyId").FromTable("Artists");
        Delete.Column("MusicBrainzId").FromTable("Artists");

        Delete.Table("MusicFiles");

        Delete.Index("IX_MediaItems_ForeignTrackId").OnTable("MediaItems");
        Delete.Index("IX_MediaItems_AlbumId").OnTable("MediaItems");
        Delete.Column("Explicit").FromTable("MediaItems");
        Delete.Column("DurationSeconds").FromTable("MediaItems");
        Delete.Column("DiscNumber").FromTable("MediaItems");
        Delete.Column("TrackNumber").FromTable("MediaItems");
        Delete.Column("MusicBrainzId").FromTable("MediaItems");
        Delete.Column("ForeignTrackId").FromTable("MediaItems");
        Delete.Column("AlbumId").FromTable("MediaItems");

        Delete.Table("Albums");
    }
}

// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(031)]
public class AddSmartListsAndDelayProfiles : FluentMigrator.Migration
{
    public override void Up()
    {
        // Smart Lists — discovery-driven auto-add lists
        Create.Table("SmartLists")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("Name").AsString(200).NotNullable()
            .WithColumn("Source").AsInt32().NotNullable()
            .WithColumn("MediaType").AsInt32().NotNullable()
            .WithColumn("QueryParametersJson").AsString(int.MaxValue).NotNullable().WithDefaultValue("{}")
            .WithColumn("QualityProfileId").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("RootFolderPath").AsString(1024).NotNullable().WithDefaultValue("")
            .WithColumn("RefreshInterval").AsInt32().NotNullable().WithDefaultValue(2) // Weekly
            .WithColumn("SearchOnAdd").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("Enabled").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("MaxItemsPerRefresh").AsInt32().NotNullable().WithDefaultValue(50)
            .WithColumn("MinimumRating").AsInt32().Nullable()
            .WithColumn("MinYear").AsInt32().Nullable()
            .WithColumn("MaxYear").AsInt32().Nullable()
            .WithColumn("ExcludeGenres").AsString(500).Nullable()
            .WithColumn("Language").AsString(10).Nullable()
            .WithColumn("Tags").AsString(500).Nullable()
            .WithColumn("ItemsAdded").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("LastRefreshed").AsDateTime().Nullable()
            .WithColumn("CreatedAt").AsDateTime().NotNullable()
            .WithColumn("UpdatedAt").AsDateTime().NotNullable();

        Create.Index("IX_SmartLists_Enabled_LastRefreshed")
            .OnTable("SmartLists")
            .OnColumn("Enabled").Ascending()
            .OnColumn("LastRefreshed").Ascending();

        // Smart List Matches — discovered items and their status
        Create.Table("SmartListMatches")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("SmartListId").AsInt32().NotNullable()
            .WithColumn("ExternalId").AsString(256).NotNullable()
            .WithColumn("MediaType").AsInt32().NotNullable()
            .WithColumn("Title").AsString(500).NotNullable()
            .WithColumn("Year").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("Rating").AsInt32().Nullable()
            .WithColumn("Status").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("MediaItemId").AsInt32().Nullable()
            .WithColumn("MetadataJson").AsString(int.MaxValue).Nullable()
            .WithColumn("DiscoveredAt").AsDateTime().NotNullable()
            .WithColumn("AddedAt").AsDateTime().Nullable();

        Create.Index("IX_SmartListMatches_SmartListId_ExternalId")
            .OnTable("SmartListMatches")
            .OnColumn("SmartListId").Ascending()
            .OnColumn("ExternalId").Ascending()
            .WithOptions().Unique();

        Create.Index("IX_SmartListMatches_SmartListId_Status")
            .OnTable("SmartListMatches")
            .OnColumn("SmartListId").Ascending()
            .OnColumn("Status").Ascending();

        Create.ForeignKey("FK_SmartListMatches_SmartListId")
            .FromTable("SmartListMatches").ForeignColumn("SmartListId")
            .ToTable("SmartLists").PrimaryColumn("Id");

        // Delay Profiles — quality-conscious acquisition delays
        Create.Table("DelayProfiles")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("Name").AsString(200).NotNullable()
            .WithColumn("MediaType").AsInt32().Nullable()
            .WithColumn("PreferredProtocol").AsInt32().NotNullable().WithDefaultValue(1) // Usenet
            .WithColumn("UsenetDelay").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("TorrentDelay").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("PreferredQualityWeight").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("BypassIfPreferredQuality").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("BypassIfPreferredWords").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("Tags").AsString(500).Nullable()
            .WithColumn("Order").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("Enabled").AsBoolean().NotNullable().WithDefaultValue(true);

        Create.Index("IX_DelayProfiles_Enabled_Order")
            .OnTable("DelayProfiles")
            .OnColumn("Enabled").Ascending()
            .OnColumn("Order").Ascending();
    }

    public override void Down()
    {
        Delete.ForeignKey("FK_SmartListMatches_SmartListId").OnTable("SmartListMatches");
        Delete.Table("SmartListMatches");
        Delete.Table("SmartLists");
        Delete.Table("DelayProfiles");
    }
}

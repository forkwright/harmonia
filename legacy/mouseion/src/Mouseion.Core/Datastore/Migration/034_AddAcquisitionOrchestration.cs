// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(034)]
public class AddAcquisitionOrchestration : FluentMigrator.Migration
{
    public override void Up()
    {
        // Debrid service credentials
        Create.Table("DebridServices")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("Name").AsString(200).NotNullable()
            .WithColumn("Provider").AsInt32().NotNullable() // RealDebrid=0, AllDebrid=1, Premiumize=2
            .WithColumn("ApiKey").AsString(500).NotNullable()
            .WithColumn("Enabled").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("Priority").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("BandwidthLimitGb").AsInt32().Nullable() // Monthly cap in GB
            .WithColumn("BandwidthUsedGb").AsDecimal().Nullable()
            .WithColumn("BandwidthResetDate").AsDateTime().Nullable()
            .WithColumn("LastChecked").AsDateTime().Nullable()
            .WithColumn("CreatedAt").AsDateTime().NotNullable();

        // .strm file tracking
        Create.Table("StrmFiles")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("MediaItemId").AsInt32().NotNullable()
            .WithColumn("DebridServiceId").AsInt32().Nullable()
            .WithColumn("FilePath").AsString(1024).NotNullable() // Local .strm file path
            .WithColumn("StreamUrl").AsString(int.MaxValue).NotNullable() // URL inside .strm
            .WithColumn("Quality").AsString(50).Nullable()
            .WithColumn("SizeBytes").AsInt64().Nullable()
            .WithColumn("IsValid").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("LastVerified").AsDateTime().Nullable()
            .WithColumn("ExpiresAt").AsDateTime().Nullable()
            .WithColumn("CreatedAt").AsDateTime().NotNullable();

        Create.Index("IX_StrmFiles_MediaItemId")
            .OnTable("StrmFiles")
            .OnColumn("MediaItemId").Ascending();

        Create.Index("IX_StrmFiles_IsValid")
            .OnTable("StrmFiles")
            .OnColumn("IsValid").Ascending();

        // Acquisition queue — priority-based work queue
        Create.Table("AcquisitionQueue")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("MediaItemId").AsInt32().NotNullable()
            .WithColumn("MediaType").AsInt32().NotNullable()
            .WithColumn("Title").AsString(500).NotNullable()
            .WithColumn("Priority").AsInt32().NotNullable().WithDefaultValue(50) // 0=highest, 100=lowest
            .WithColumn("Strategy").AsInt32().NotNullable().WithDefaultValue(0) // Download=0, Strm=1, MonitorOnly=2
            .WithColumn("Status").AsInt32().NotNullable().WithDefaultValue(0) // Queued=0, Searching=1, Found=2, Grabbing=3, Complete=4, Failed=5, Skipped=6
            .WithColumn("Source").AsInt32().NotNullable().WithDefaultValue(0) // UserTriggered=0, RssSync=1, SmartList=2, Import=3
            .WithColumn("QualityProfileId").AsInt32().Nullable()
            .WithColumn("PreferredIndexers").AsString(500).Nullable() // JSON array of indexer IDs
            .WithColumn("ErrorMessage").AsString(int.MaxValue).Nullable()
            .WithColumn("RetryCount").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("MaxRetries").AsInt32().NotNullable().WithDefaultValue(3)
            .WithColumn("NextRetryAt").AsDateTime().Nullable()
            .WithColumn("RequestedBy").AsInt32().Nullable() // User ID
            .WithColumn("RequestedAt").AsDateTime().NotNullable()
            .WithColumn("StartedAt").AsDateTime().Nullable()
            .WithColumn("CompletedAt").AsDateTime().Nullable();

        Create.Index("IX_AcquisitionQueue_Status_Priority")
            .OnTable("AcquisitionQueue")
            .OnColumn("Status").Ascending()
            .OnColumn("Priority").Ascending();

        Create.Index("IX_AcquisitionQueue_MediaItemId")
            .OnTable("AcquisitionQueue")
            .OnColumn("MediaItemId").Ascending();

        // Acquisition log — full audit trail
        Create.Table("AcquisitionLog")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("QueueItemId").AsInt32().Nullable()
            .WithColumn("MediaItemId").AsInt32().NotNullable()
            .WithColumn("Action").AsString(50).NotNullable() // searched, found, grabbed, skipped, failed, strm_created, strm_expired
            .WithColumn("IndexerName").AsString(200).Nullable()
            .WithColumn("ReleaseName").AsString(500).Nullable()
            .WithColumn("Quality").AsString(50).Nullable()
            .WithColumn("SizeBytes").AsInt64().Nullable()
            .WithColumn("Reason").AsString(500).Nullable() // Why this action was taken
            .WithColumn("DetailsJson").AsString(int.MaxValue).Nullable()
            .WithColumn("Timestamp").AsDateTime().NotNullable();

        Create.Index("IX_AcquisitionLog_Timestamp")
            .OnTable("AcquisitionLog")
            .OnColumn("Timestamp").Descending();

        Create.Index("IX_AcquisitionLog_MediaItemId")
            .OnTable("AcquisitionLog")
            .OnColumn("MediaItemId").Ascending();

        Create.Index("IX_AcquisitionLog_Action")
            .OnTable("AcquisitionLog")
            .OnColumn("Action").Ascending();
    }

    public override void Down()
    {
        Delete.Table("AcquisitionLog");
        Delete.Table("AcquisitionQueue");
        Delete.Table("StrmFiles");
        Delete.Table("DebridServices");
    }
}

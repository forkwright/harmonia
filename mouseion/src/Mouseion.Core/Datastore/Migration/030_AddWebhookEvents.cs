// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(030)]
public class AddWebhookEvents : FluentMigrator.Migration
{
    public override void Up()
    {
        Create.Table("WebhookEvents")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("Source").AsInt32().NotNullable()
            .WithColumn("EventType").AsString(50).NotNullable()
            .WithColumn("ExternalItemId").AsString(256).NotNullable()
            .WithColumn("ExternalUserId").AsString(256).Nullable()
            .WithColumn("ResolvedMediaItemId").AsInt32().Nullable()
            .WithColumn("RawPayload").AsString(int.MaxValue).NotNullable()
            .WithColumn("Processed").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("Error").AsString(1024).Nullable()
            .WithColumn("ReceivedAt").AsDateTime().NotNullable()
            .WithColumn("ProcessedAt").AsDateTime().Nullable();

        Create.Index("IX_WebhookEvents_Source_ExternalItemId_EventType_ReceivedAt")
            .OnTable("WebhookEvents")
            .OnColumn("Source").Ascending()
            .OnColumn("ExternalItemId").Ascending()
            .OnColumn("EventType").Ascending()
            .OnColumn("ReceivedAt").Descending();

        Create.Index("IX_WebhookEvents_Processed")
            .OnTable("WebhookEvents")
            .OnColumn("Processed").Ascending()
            .OnColumn("ReceivedAt").Ascending();
    }

    public override void Down()
    {
        Delete.Table("WebhookEvents");
    }
}

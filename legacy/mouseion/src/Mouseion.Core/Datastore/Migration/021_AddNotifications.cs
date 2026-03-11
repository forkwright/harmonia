// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(21, "Add Notifications table")]
public class Migration021AddNotifications : FluentMigrator.Migration
{
    public override void Up()
    {
        Create.Table("Notifications")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("Name").AsString().NotNullable()
            .WithColumn("Implementation").AsString().NotNullable()
            .WithColumn("ConfigContract").AsString().NotNullable()
            .WithColumn("Settings").AsString().Nullable()
            .WithColumn("Enabled").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("OnGrab").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("OnDownload").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("OnRename").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("OnMediaAdded").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("OnMediaDeleted").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("OnHealthIssue").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("OnHealthRestored").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("OnApplicationUpdate").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("Tags").AsString().Nullable();

        Create.Index("IX_Notifications_Enabled")
            .OnTable("Notifications")
            .OnColumn("Enabled");

        Create.Index("IX_Notifications_Implementation")
            .OnTable("Notifications")
            .OnColumn("Implementation");
    }

    public override void Down()
    {
        Delete.Table("Notifications");
    }
}

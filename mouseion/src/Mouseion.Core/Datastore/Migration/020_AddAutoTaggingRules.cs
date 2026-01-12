// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(20, "Add AutoTaggingRules table")]
public class Migration020AddAutoTaggingRules : FluentMigrator.Migration
{
    public override void Up()
    {
        Create.Table("AutoTaggingRules")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("Name").AsString().NotNullable()
            .WithColumn("Enabled").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("ConditionType").AsInt32().NotNullable()
            .WithColumn("ConditionValue").AsString().NotNullable()
            .WithColumn("TagId").AsInt32().NotNullable()
            .WithColumn("MediaTypeFilter").AsInt32().Nullable();

        Create.Index("IX_AutoTaggingRules_TagId")
            .OnTable("AutoTaggingRules")
            .OnColumn("TagId");

        Create.Index("IX_AutoTaggingRules_Enabled")
            .OnTable("AutoTaggingRules")
            .OnColumn("Enabled");
    }

    public override void Down()
    {
        Delete.Table("AutoTaggingRules");
    }
}

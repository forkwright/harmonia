// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(033)]
public class AddUserPreferencesAndPermissions : FluentMigrator.Migration
{
    public override void Up()
    {
        // Per-user preferences: hidden media types, quality overrides, UI settings
        Create.Table("UserPreferences")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("UserId").AsInt32().NotNullable().Unique()
            .WithColumn("HiddenMediaTypes").AsString(500).Nullable() // JSON array: [7, 8] = hide Comic + Manga
            .WithColumn("DefaultQualityProfileId").AsInt32().Nullable() // Override server default
            .WithColumn("QualityProfileOverrides").AsString(int.MaxValue).Nullable() // JSON: {mediaType: profileId}
            .WithColumn("Language").AsString(10).Nullable()
            .WithColumn("Theme").AsString(50).Nullable()
            .WithColumn("NotificationsEnabled").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("CreatedAt").AsDateTime().NotNullable()
            .WithColumn("UpdatedAt").AsDateTime().NotNullable();

        Create.ForeignKey("FK_UserPreferences_UserId")
            .FromTable("UserPreferences").ForeignColumn("UserId")
            .ToTable("Users").PrimaryColumn("Id");

        // Per-user smart list subscriptions
        Create.Table("UserSmartListSubscriptions")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("UserId").AsInt32().NotNullable()
            .WithColumn("SmartListId").AsInt32().NotNullable()
            .WithColumn("AutoAdd").AsBoolean().NotNullable().WithDefaultValue(false) // Auto-add to user's watchlist
            .WithColumn("NotifyOnNew").AsBoolean().NotNullable().WithDefaultValue(true) // Notify on new matches
            .WithColumn("SubscribedAt").AsDateTime().NotNullable();

        Create.Index("IX_UserSmartListSubscriptions_UserId_SmartListId")
            .OnTable("UserSmartListSubscriptions")
            .OnColumn("UserId").Ascending()
            .OnColumn("SmartListId").Ascending()
            .WithOptions().Unique();

        Create.ForeignKey("FK_UserSmartListSubscriptions_UserId")
            .FromTable("UserSmartListSubscriptions").ForeignColumn("UserId")
            .ToTable("Users").PrimaryColumn("Id");

        Create.ForeignKey("FK_UserSmartListSubscriptions_SmartListId")
            .FromTable("UserSmartListSubscriptions").ForeignColumn("SmartListId")
            .ToTable("SmartLists").PrimaryColumn("Id");

        // Resource-level permissions: restrict users to specific media types or root folders
        Create.Table("UserPermissions")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("UserId").AsInt32().NotNullable()
            .WithColumn("PermissionType").AsInt32().NotNullable() // MediaTypeAccess=0, RootFolderAccess=1, DownloadPermission=2
            .WithColumn("ResourceId").AsString(256).NotNullable() // Media type int or root folder path
            .WithColumn("Allowed").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("GrantedBy").AsInt32().Nullable() // Admin user who granted
            .WithColumn("GrantedAt").AsDateTime().NotNullable();

        Create.Index("IX_UserPermissions_UserId_PermissionType")
            .OnTable("UserPermissions")
            .OnColumn("UserId").Ascending()
            .OnColumn("PermissionType").Ascending();

        Create.ForeignKey("FK_UserPermissions_UserId")
            .FromTable("UserPermissions").ForeignColumn("UserId")
            .ToTable("Users").PrimaryColumn("Id");

        // Scoped API keys: tied to a user with that user's permissions
        Create.Table("ApiKeys")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("UserId").AsInt32().NotNullable()
            .WithColumn("Name").AsString(200).NotNullable() // "OPDS Reader", "Automation Script"
            .WithColumn("KeyHash").AsString(128).NotNullable() // bcrypt hash of key
            .WithColumn("KeyPrefix").AsString(8).NotNullable() // First 8 chars for identification
            .WithColumn("Scopes").AsString(500).Nullable() // JSON array: ["read", "progress", "download"]
            .WithColumn("ExpiresAt").AsDateTime().Nullable()
            .WithColumn("LastUsedAt").AsDateTime().Nullable()
            .WithColumn("IsActive").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("CreatedAt").AsDateTime().NotNullable();

        Create.Index("IX_ApiKeys_KeyPrefix")
            .OnTable("ApiKeys")
            .OnColumn("KeyPrefix").Ascending();

        Create.Index("IX_ApiKeys_UserId")
            .OnTable("ApiKeys")
            .OnColumn("UserId").Ascending();

        Create.ForeignKey("FK_ApiKeys_UserId")
            .FromTable("ApiKeys").ForeignColumn("UserId")
            .ToTable("Users").PrimaryColumn("Id");

        // Audit log: authentication events, permission changes, admin actions
        Create.Table("AuditLog")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("UserId").AsInt32().Nullable() // null for unauthenticated events
            .WithColumn("Action").AsString(100).NotNullable() // "login", "login_failed", "permission_change", "user_created", etc.
            .WithColumn("ResourceType").AsString(100).Nullable() // "User", "ApiKey", "Permission"
            .WithColumn("ResourceId").AsString(256).Nullable()
            .WithColumn("Details").AsString(int.MaxValue).Nullable() // JSON context
            .WithColumn("IpAddress").AsString(45).Nullable() // IPv4 or IPv6
            .WithColumn("UserAgent").AsString(500).Nullable()
            .WithColumn("Timestamp").AsDateTime().NotNullable();

        Create.Index("IX_AuditLog_Timestamp")
            .OnTable("AuditLog")
            .OnColumn("Timestamp").Descending();

        Create.Index("IX_AuditLog_UserId_Timestamp")
            .OnTable("AuditLog")
            .OnColumn("UserId").Ascending()
            .OnColumn("Timestamp").Descending();

        Create.Index("IX_AuditLog_Action")
            .OnTable("AuditLog")
            .OnColumn("Action").Ascending();
    }

    public override void Down()
    {
        Delete.ForeignKey("FK_UserPreferences_UserId").OnTable("UserPreferences");
        Delete.ForeignKey("FK_UserSmartListSubscriptions_UserId").OnTable("UserSmartListSubscriptions");
        Delete.ForeignKey("FK_UserSmartListSubscriptions_SmartListId").OnTable("UserSmartListSubscriptions");
        Delete.ForeignKey("FK_UserPermissions_UserId").OnTable("UserPermissions");
        Delete.ForeignKey("FK_ApiKeys_UserId").OnTable("ApiKeys");
        Delete.Table("AuditLog");
        Delete.Table("ApiKeys");
        Delete.Table("UserPermissions");
        Delete.Table("UserSmartListSubscriptions");
        Delete.Table("UserPreferences");
    }
}

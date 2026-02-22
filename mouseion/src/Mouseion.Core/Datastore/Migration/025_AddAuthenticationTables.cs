// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(25, "Add authentication tables and migrate UserId references")]
public class Migration025AddAuthenticationTables : FluentMigrator.Migration
{
    public override void Up()
    {
        Create.Table("Users")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("Username").AsString(100).NotNullable()
            .WithColumn("DisplayName").AsString(200).NotNullable()
            .WithColumn("Email").AsString(254).NotNullable().WithDefaultValue("")
            .WithColumn("Role").AsInt32().NotNullable().WithDefaultValue(1) // UserRole.User
            .WithColumn("AuthenticationMethod").AsString(50).NotNullable().WithDefaultValue("local")
            .WithColumn("PasswordHash").AsString(500).NotNullable().WithDefaultValue("")
            .WithColumn("IsActive").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("CreatedAt").AsDateTime().NotNullable()
            .WithColumn("UpdatedAt").AsDateTime().NotNullable()
            .WithColumn("LastLoginAt").AsDateTime().Nullable();

        Create.Table("RefreshTokens")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("UserId").AsInt32().NotNullable()
            .WithColumn("Token").AsString(500).NotNullable()
            .WithColumn("DeviceName").AsString(200).NotNullable().WithDefaultValue("")
            .WithColumn("ExpiresAt").AsDateTime().NotNullable()
            .WithColumn("CreatedAt").AsDateTime().NotNullable()
            .WithColumn("RevokedAt").AsDateTime().Nullable();

        // Users indexes
        Create.Index("IX_Users_Username")
            .OnTable("Users")
            .OnColumn("Username")
            .Unique();

        Create.Index("IX_Users_Email")
            .OnTable("Users")
            .OnColumn("Email");

        Create.Index("IX_Users_IsActive")
            .OnTable("Users")
            .OnColumn("IsActive");

        // RefreshTokens indexes
        Create.Index("IX_RefreshTokens_Token")
            .OnTable("RefreshTokens")
            .OnColumn("Token")
            .Unique();

        Create.Index("IX_RefreshTokens_UserId")
            .OnTable("RefreshTokens")
            .OnColumn("UserId");

        Create.Index("IX_RefreshTokens_ExpiresAt")
            .OnTable("RefreshTokens")
            .OnColumn("ExpiresAt");

        // Seed default admin user (password will be set on first login or via config)
        // The application will check for this on startup and create if missing
    }

    public override void Down()
    {
        Delete.Table("RefreshTokens");
        Delete.Table("Users");
    }
}

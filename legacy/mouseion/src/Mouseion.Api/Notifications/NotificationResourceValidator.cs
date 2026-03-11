// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentValidation;

namespace Mouseion.Api.Notifications;

public class NotificationResourceValidator : AbstractValidator<NotificationResource>
{
    private static readonly HashSet<string> ValidTypes = new(StringComparer.OrdinalIgnoreCase)
    {
        "Discord",
        "Slack",
        "Telegram",
        "Email",
        "Webhook",
        "Pushover",
        "Gotify",
        "Ntfy",
        "Apprise"
    };

    public NotificationResourceValidator()
    {
        RuleFor(x => x.Name)
            .NotEmpty().WithMessage("Name is required")
            .MaximumLength(200).WithMessage("Name must not exceed 200 characters");

        RuleFor(x => x.Type)
            .NotEmpty().WithMessage("Type is required")
            .Must(type => ValidTypes.Contains(type))
            .WithMessage($"Type must be one of: {string.Join(", ", ValidTypes)}");

        RuleFor(x => x.Settings)
            .NotNull().WithMessage("Settings are required");

        RuleFor(x => x)
            .Must(x => x.OnGrab || x.OnDownload || x.OnRename || x.OnMediaAdded ||
                       x.OnMediaDeleted || x.OnHealthIssue || x.OnHealthRestored || x.OnApplicationUpdate)
            .WithMessage("At least one notification trigger must be enabled");
    }
}

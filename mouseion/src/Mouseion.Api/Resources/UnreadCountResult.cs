// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Api.Resources;

/// <summary>
/// Shared result for unread count queries across media types.
/// </summary>
public class UnreadCountResult
{
    public int Count { get; set; }
}

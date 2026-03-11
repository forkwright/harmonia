// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Api.Search;

public class SearchRequest
{
    public string? Q { get; set; }
    public int Limit { get; set; } = 50;
}

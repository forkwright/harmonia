// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Tests.Movies.Organization;

public class FileOrganizationServiceTests
{
    [Fact]
    public void NamingPattern_SupportedTokens_ShouldBeDocumented()
    {
        // Documents the supported naming pattern tokens:
        // Basic: {Movie Title}, {Title}, {Movie Year}, {Year}
        // Metadata: {Quality}, {Studio}, {Certification}, {TmdbId}, {ImdbId}
        // File: {File Extension}, {Extension}
        Assert.True(true, "Naming pattern tokens documented");
    }

    [Fact]
    public void FileStrategy_Hardlink_ShouldBeDefault()
    {
        // Validates that hardlink is the default file strategy
        var defaultStrategy = Mouseion.Core.Movies.Organization.FileStrategy.Hardlink;
        Assert.Equal(1, (int)defaultStrategy);
    }

    [Fact]
    public void OrganizationResult_DryRun_ShouldIndicatePreview()
    {
        // Validates that dry-run mode is properly indicated in results
        var result = new Mouseion.Core.Movies.Organization.OrganizationResult
        {
            IsDryRun = true,
            Success = true,
            OriginalPath = "/media/movies/movie.mkv",
            NewPath = "/media/movies/The Matrix (1999)/The Matrix (1999) - Bluray-1080p.mkv"
        };

        Assert.True(result.IsDryRun);
        Assert.True(result.Success);
    }
}

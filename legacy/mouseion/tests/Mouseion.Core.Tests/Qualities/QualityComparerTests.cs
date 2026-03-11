// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Qualities;

namespace Mouseion.Core.Tests.Qualities;

public class QualityComparerTests
{
    [Fact]
    public void Compare_should_return_positive_when_left_quality_is_better()
    {
        var lower = new QualityModel { Quality = Quality.HDTV720p, Revision = new Revision(1) };
        var higher = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) };

        var result = QualityComparer.Compare(higher, lower);

        Assert.True(result > 0);
    }

    [Fact]
    public void Compare_should_return_negative_when_left_quality_is_worse()
    {
        var lower = new QualityModel { Quality = Quality.HDTV720p, Revision = new Revision(1) };
        var higher = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) };

        var result = QualityComparer.Compare(lower, higher);

        Assert.True(result < 0);
    }

    [Fact]
    public void Compare_should_return_zero_when_qualities_and_revisions_are_equal()
    {
        var left = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) };
        var right = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) };

        var result = QualityComparer.Compare(left, right);

        Assert.Equal(0, result);
    }

    [Fact]
    public void Compare_should_use_revision_when_qualities_are_equal()
    {
        var v1 = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) };
        var v2 = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(2) };

        var result = QualityComparer.Compare(v2, v1);

        Assert.True(result > 0); // v2 > v1
    }

    [Fact]
    public void Compare_should_prioritize_quality_over_revision()
    {
        var hdtv720pV2 = new QualityModel { Quality = Quality.HDTV720p, Revision = new Revision(2) };
        var bluray1080pV1 = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) };

        var result = QualityComparer.Compare(bluray1080pV1, hdtv720pV2);

        Assert.True(result > 0); // Bluray1080p v1 > HDTV720p v2
    }

    [Fact]
    public void IsUpgrade_should_return_true_when_candidate_is_better()
    {
        var current = new QualityModel { Quality = Quality.HDTV720p, Revision = new Revision(1) };
        var candidate = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) };

        var result = QualityComparer.IsUpgrade(current, candidate);

        Assert.True(result);
    }

    [Fact]
    public void IsUpgrade_should_return_false_when_candidate_is_same()
    {
        var current = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) };
        var candidate = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) };

        var result = QualityComparer.IsUpgrade(current, candidate);

        Assert.False(result);
    }

    [Fact]
    public void IsUpgrade_should_return_false_when_candidate_is_worse()
    {
        var current = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) };
        var candidate = new QualityModel { Quality = Quality.HDTV720p, Revision = new Revision(1) };

        var result = QualityComparer.IsUpgrade(current, candidate);

        Assert.False(result);
    }

    [Fact]
    public void IsUpgrade_should_return_true_when_current_is_null()
    {
        var candidate = new QualityModel { Quality = Quality.HDTV720p, Revision = new Revision(1) };

        var result = QualityComparer.IsUpgrade(null, candidate);

        Assert.True(result);
    }

    [Fact]
    public void IsUpgrade_should_consider_revision_upgrades()
    {
        var current = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) };
        var candidate = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(2) };

        var result = QualityComparer.IsUpgrade(current, candidate);

        Assert.True(result); // PROPER is an upgrade
    }

    [Fact]
    public void MeetsMinimum_should_return_true_when_quality_is_higher()
    {
        var minimum = new QualityModel { Quality = Quality.HDTV720p, Revision = new Revision(1) };
        var candidate = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) };

        var result = QualityComparer.MeetsMinimum(candidate, minimum);

        Assert.True(result);
    }

    [Fact]
    public void MeetsMinimum_should_return_true_when_quality_is_equal()
    {
        var minimum = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) };
        var candidate = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) };

        var result = QualityComparer.MeetsMinimum(candidate, minimum);

        Assert.True(result);
    }

    [Fact]
    public void MeetsMinimum_should_return_false_when_quality_is_lower()
    {
        var minimum = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) };
        var candidate = new QualityModel { Quality = Quality.HDTV720p, Revision = new Revision(1) };

        var result = QualityComparer.MeetsMinimum(candidate, minimum);

        Assert.False(result);
    }

    [Fact]
    public void HasReachedCutoff_should_return_true_when_quality_exceeds_cutoff()
    {
        var cutoff = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) };
        var candidate = new QualityModel { Quality = Quality.Bluray2160p, Revision = new Revision(1) };

        var result = QualityComparer.HasReachedCutoff(candidate, cutoff);

        Assert.True(result);
    }

    [Fact]
    public void HasReachedCutoff_should_return_true_when_quality_equals_cutoff()
    {
        var cutoff = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) };
        var candidate = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) };

        var result = QualityComparer.HasReachedCutoff(candidate, cutoff);

        Assert.True(result);
    }

    [Fact]
    public void HasReachedCutoff_should_return_false_when_quality_below_cutoff()
    {
        var cutoff = new QualityModel { Quality = Quality.Bluray1080p, Revision = new Revision(1) };
        var candidate = new QualityModel { Quality = Quality.HDTV720p, Revision = new Revision(1) };

        var result = QualityComparer.HasReachedCutoff(candidate, cutoff);

        Assert.False(result);
    }

    [Theory]
    [InlineData(1, 2, -1)] // Weight 1 < Weight 2
    [InlineData(2, 1, 1)]  // Weight 2 > Weight 1
    [InlineData(1, 1, 0)]  // Weight 1 = Weight 1
    public void Compare_should_respect_weight_ordering(int weight1Id, int weight2Id, int expected)
    {
        // Use actual quality IDs that correspond to known weights
        var quality1 = weight1Id == 1 ? Quality.Unknown : Quality.CAM;
        var quality2 = weight2Id == 1 ? Quality.Unknown : weight2Id == 2 ? Quality.WORKPRINT : Quality.CAM;

        var q1 = new QualityModel { Quality = quality1, Revision = new Revision(1) };
        var q2 = new QualityModel { Quality = quality2, Revision = new Revision(1) };

        var result = QualityComparer.Compare(q1, q2);

        if (expected > 0)
        {
            Assert.True(result > 0);
        }
        else if (expected < 0)
        {
            Assert.True(result < 0);
        }
        else
        {
            Assert.Equal(0, result);
        }
    }

    [Fact]
    public void Compare_should_handle_unknown_quality()
    {
        var unknown = new QualityModel { Quality = Quality.Unknown, Revision = new Revision(1) };
        var known = new QualityModel { Quality = Quality.HDTV720p, Revision = new Revision(1) };

        var result = QualityComparer.Compare(known, unknown);

        Assert.True(result > 0); // Any known quality > Unknown
    }
}

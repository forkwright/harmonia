// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Common.Extensions;

namespace Mouseion.Common.Tests.Extensions;

public class LevenstheinExtensionsTests
{
    [Fact]
    public void LevenshteinDistance_should_return_zero_for_identical_strings()
    {
        var result = "hello".LevenshteinDistance("hello");

        Assert.Equal(0, result);
    }

    [Fact]
    public void LevenshteinDistance_should_return_length_for_empty_first_string()
    {
        var result = "".LevenshteinDistance("hello");

        Assert.Equal(5, result);
    }

    [Fact]
    public void LevenshteinDistance_should_return_length_for_empty_second_string()
    {
        var result = "hello".LevenshteinDistance("");

        Assert.Equal(5, result);
    }

    [Fact]
    public void LevenshteinDistance_should_calculate_single_insertion()
    {
        var result = "hello".LevenshteinDistance("helloo");

        Assert.Equal(1, result);
    }

    [Fact]
    public void LevenshteinDistance_should_calculate_single_deletion()
    {
        var result = "hello".LevenshteinDistance("helo");

        Assert.Equal(1, result);
    }

    [Fact]
    public void LevenshteinDistance_should_calculate_single_substitution()
    {
        var result = "hello".LevenshteinDistance("hallo");

        Assert.Equal(1, result);
    }

    [Fact]
    public void LevenshteinDistance_should_calculate_multiple_edits()
    {
        var result = "kitten".LevenshteinDistance("sitting");

        Assert.Equal(3, result);
    }

    [Fact]
    public void LevenshteinDistance_should_use_custom_insert_cost()
    {
        var result = "cat".LevenshteinDistance("cats", costInsert: 2);

        Assert.Equal(2, result);
    }

    [Fact]
    public void LevenshteinDistance_should_use_custom_delete_cost()
    {
        var result = "cats".LevenshteinDistance("cat", costDelete: 3);

        Assert.Equal(3, result);
    }

    [Fact]
    public void LevenshteinDistance_should_use_custom_substitute_cost()
    {
        var result = "cat".LevenshteinDistance("bat", costSubstitute: 4);

        Assert.Equal(4, result);
    }

    [Fact]
    public void LevenshteinDistanceClean_should_be_case_insensitive()
    {
        var result = "Hello".LevenshteinDistanceClean("HELLO");

        Assert.Equal(0, result);
    }

    [Fact]
    public void LevenshteinDistanceClean_should_ignore_periods()
    {
        var result = "H.E.L.L.O.".LevenshteinDistanceClean("hello");

        Assert.Equal(0, result);
    }

    [Fact]
    public void LevenshteinDistanceClean_should_use_weighted_costs()
    {
        // LevenshteinDistanceClean uses costInsert=1, costDelete=3, costSubstitute=3
        var result = "abc".LevenshteinDistanceClean("ab");

        // Deletion costs 3
        Assert.Equal(3, result);
    }

    [Fact]
    public void LevenshteinDistance_should_handle_unicode_characters()
    {
        var result = "café".LevenshteinDistance("cafe");

        Assert.Equal(1, result);
    }

    [Fact]
    public void LevenshteinDistance_should_handle_completely_different_strings()
    {
        var result = "abc".LevenshteinDistance("xyz");

        Assert.Equal(3, result);
    }
}

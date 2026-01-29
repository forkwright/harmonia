// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.RegularExpressions;
using Mouseion.Common.Extensions;

namespace Mouseion.Common.Tests.Extensions;

public class RegexExtensionsTests
{
    [Fact]
    public void EndIndex_should_return_correct_end_position()
    {
        var regex = new Regex("world");
        var match = regex.Match("hello world");

        var endIndex = match.EndIndex();

        Assert.Equal(11, endIndex); // "hello world" -> "world" starts at 6, length 5, ends at 11
    }

    [Fact]
    public void EndIndex_should_work_for_match_at_start()
    {
        var regex = new Regex("hello");
        var match = regex.Match("hello world");

        var endIndex = match.EndIndex();

        Assert.Equal(5, endIndex);
    }

    [Fact]
    public void EndIndex_should_work_for_single_character_match()
    {
        var regex = new Regex("o");
        var match = regex.Match("hello");

        var endIndex = match.EndIndex();

        Assert.Equal(5, endIndex); // First 'o' is at index 4, length 1
    }

    [Fact]
    public void EndIndex_should_work_for_empty_match()
    {
        var regex = new Regex("");
        var match = regex.Match("hello");

        var endIndex = match.EndIndex();

        Assert.Equal(0, endIndex); // Empty match at index 0, length 0
    }

    [Fact]
    public void EndIndex_should_work_with_groups()
    {
        var regex = new Regex(@"(\w+)@(\w+)\.(\w+)");
        var match = regex.Match("test@example.com");

        // Check the first group
        var group1EndIndex = match.Groups[1].EndIndex();
        Assert.Equal(4, group1EndIndex); // "test" ends at 4

        // Check the second group
        var group2EndIndex = match.Groups[2].EndIndex();
        Assert.Equal(12, group2EndIndex); // "example" ends at 12

        // Check the third group
        var group3EndIndex = match.Groups[3].EndIndex();
        Assert.Equal(16, group3EndIndex); // "com" ends at 16
    }

    [Fact]
    public void EndIndex_should_work_for_match_at_end()
    {
        var regex = new Regex("world$");
        var match = regex.Match("hello world");

        var endIndex = match.EndIndex();

        Assert.Equal(11, endIndex);
    }

    [Fact]
    public void EndIndex_should_calculate_correctly_for_unicode()
    {
        var regex = new Regex("café");
        var match = regex.Match("I love café");

        var endIndex = match.EndIndex();

        Assert.Equal(11, endIndex); // "café" starts at 7, length 4
    }
}

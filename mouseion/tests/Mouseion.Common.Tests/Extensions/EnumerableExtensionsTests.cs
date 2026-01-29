// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Common.Extensions;

namespace Mouseion.Common.Tests.Extensions;

public class EnumerableExtensionsTests
{
    [Fact]
    public void IntersectBy_should_return_matching_elements()
    {
        var first = new[] { ("a", 1), ("b", 2), ("c", 3) };
        var second = new[] { 2, 3, 4 };

        var result = first.IntersectBy(
            x => x.Item2,
            second,
            x => x,
            EqualityComparer<int>.Default).ToList();

        Assert.Equal(2, result.Count);
        Assert.Contains(("b", 2), result);
        Assert.Contains(("c", 3), result);
    }

    [Fact]
    public void IntersectBy_should_return_empty_when_no_matches()
    {
        var first = new[] { ("a", 1), ("b", 2) };
        var second = new[] { 3, 4 };

        var result = first.IntersectBy(
            x => x.Item2,
            second,
            x => x,
            EqualityComparer<int>.Default).ToList();

        Assert.Empty(result);
    }

    [Fact]
    public void ExceptBy_should_return_elements_not_in_second()
    {
        var first = new[] { ("a", 1), ("b", 2), ("c", 3) };
        var second = new[] { 2 };

        var result = first.ExceptBy(
            x => x.Item2,
            second,
            x => x,
            EqualityComparer<int>.Default).ToList();

        Assert.Equal(2, result.Count);
        Assert.Contains(("a", 1), result);
        Assert.Contains(("c", 3), result);
    }

    [Fact]
    public void ToDictionaryIgnoreDuplicates_should_keep_first_on_duplicates()
    {
        var source = new[] { ("a", 1), ("a", 2), ("b", 3) };

        var result = source.ToDictionaryIgnoreDuplicates(x => x.Item1);

        Assert.Equal(2, result.Count);
        Assert.Equal(("a", 1), result["a"]);
        Assert.Equal(("b", 3), result["b"]);
    }

    [Fact]
    public void ToDictionaryIgnoreDuplicates_with_value_selector_should_work()
    {
        var source = new[] { ("a", 1), ("a", 2), ("b", 3) };

        var result = source.ToDictionaryIgnoreDuplicates(x => x.Item1, x => x.Item2);

        Assert.Equal(2, result.Count);
        Assert.Equal(1, result["a"]);
        Assert.Equal(3, result["b"]);
    }

    [Fact]
    public void AddIfNotNull_should_add_non_null_item()
    {
        var list = new List<string>();

        list.AddIfNotNull("test");

        Assert.Single(list);
        Assert.Equal("test", list[0]);
    }

    [Fact]
    public void AddIfNotNull_should_not_add_null_item()
    {
        var list = new List<string?>();

        list.AddIfNotNull(null);

        Assert.Empty(list);
    }

    [Fact]
    public void Empty_should_return_true_for_empty_collection()
    {
        var empty = Array.Empty<int>();

        Assert.True(empty.Empty());
    }

    [Fact]
    public void Empty_should_return_false_for_non_empty_collection()
    {
        var nonEmpty = new[] { 1, 2, 3 };

        Assert.False(nonEmpty.Empty());
    }

    [Fact]
    public void None_should_return_true_when_no_elements_match()
    {
        var numbers = new[] { 1, 2, 3 };

        Assert.True(numbers.None(x => x > 10));
    }

    [Fact]
    public void None_should_return_false_when_any_element_matches()
    {
        var numbers = new[] { 1, 2, 3 };

        Assert.False(numbers.None(x => x > 2));
    }

    [Fact]
    public void NotAll_should_return_true_when_some_dont_match()
    {
        var numbers = new[] { 1, 2, 3 };

        Assert.True(numbers.NotAll(x => x > 1));
    }

    [Fact]
    public void NotAll_should_return_false_when_all_match()
    {
        var numbers = new[] { 1, 2, 3 };

        Assert.False(numbers.NotAll(x => x > 0));
    }

    [Fact]
    public void SelectList_should_return_list()
    {
        var numbers = new[] { 1, 2, 3 };

        var result = numbers.SelectList(x => x * 2);

        Assert.IsType<List<int>>(result);
        Assert.Equal(new[] { 2, 4, 6 }, result);
    }

    [Fact]
    public void DropLast_should_remove_last_n_elements()
    {
        var numbers = new[] { 1, 2, 3, 4, 5 };

        var result = numbers.DropLast(2).ToList();

        Assert.Equal(new[] { 1, 2, 3 }, result);
    }

    [Fact]
    public void DropLast_should_return_empty_when_n_exceeds_count()
    {
        var numbers = new[] { 1, 2 };

        var result = numbers.DropLast(5).ToList();

        Assert.Empty(result);
    }

    [Fact]
    public void DropLast_should_throw_for_null_source()
    {
        IEnumerable<int>? source = null;

        Assert.Throws<ArgumentNullException>(() => source!.DropLast(1).ToList());
    }

    [Fact]
    public void DropLast_should_throw_for_negative_n()
    {
        var numbers = new[] { 1, 2, 3 };

        Assert.Throws<ArgumentOutOfRangeException>(() => numbers.DropLast(-1).ToList());
    }

    [Fact]
    public void ConcatToString_should_join_with_default_separator()
    {
        var items = new[] { 1, 2, 3 };

        var result = items.ConcatToString();

        Assert.Equal("1, 2, 3", result);
    }

    [Fact]
    public void ConcatToString_should_join_with_custom_separator()
    {
        var items = new[] { 1, 2, 3 };

        var result = items.ConcatToString(" | ");

        Assert.Equal("1 | 2 | 3", result);
    }

    [Fact]
    public void ConcatToString_with_predicate_should_transform_and_join()
    {
        var items = new[] { "a", "bb", "ccc" };

        var result = items.ConcatToString(x => x.Length.ToString(), "-");

        Assert.Equal("1-2-3", result);
    }
}

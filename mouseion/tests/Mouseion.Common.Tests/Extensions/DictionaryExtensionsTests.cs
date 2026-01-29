// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Common.Extensions;

namespace Mouseion.Common.Tests.Extensions;

public class DictionaryExtensionsTests
{
    [Fact]
    public void Merge_should_combine_two_dictionaries()
    {
        var first = new Dictionary<string, int> { { "a", 1 }, { "b", 2 } };
        var second = new Dictionary<string, int> { { "c", 3 }, { "d", 4 } };

        var result = first.Merge(second);

        Assert.Equal(4, result.Count);
        Assert.Equal(1, result["a"]);
        Assert.Equal(2, result["b"]);
        Assert.Equal(3, result["c"]);
        Assert.Equal(4, result["d"]);
    }

    [Fact]
    public void Merge_should_prefer_second_on_duplicate_keys()
    {
        var first = new Dictionary<string, int> { { "a", 1 }, { "b", 2 } };
        var second = new Dictionary<string, int> { { "b", 20 }, { "c", 3 } };

        var result = first.Merge(second);

        Assert.Equal(3, result.Count);
        Assert.Equal(1, result["a"]);
        Assert.Equal(20, result["b"]);
        Assert.Equal(3, result["c"]);
    }

    [Fact]
    public void Merge_should_throw_when_first_is_null()
    {
        Dictionary<string, int>? first = null;
        var second = new Dictionary<string, int> { { "a", 1 } };

        Assert.Throws<ArgumentNullException>(() => first!.Merge(second));
    }

    [Fact]
    public void Merge_should_throw_when_second_is_null()
    {
        var first = new Dictionary<string, int> { { "a", 1 } };
        Dictionary<string, int>? second = null;

        Assert.Throws<ArgumentNullException>(() => first.Merge(second!));
    }

    [Fact]
    public void Merge_should_handle_empty_dictionaries()
    {
        var first = new Dictionary<string, int>();
        var second = new Dictionary<string, int> { { "a", 1 } };

        var result = first.Merge(second);

        Assert.Single(result);
        Assert.Equal(1, result["a"]);
    }

    [Fact]
    public void Add_extension_should_add_key_value_pair()
    {
        var collection = new List<KeyValuePair<string, int>>();

        collection.Add("key", 42);

        Assert.Single(collection);
        Assert.Equal("key", collection[0].Key);
        Assert.Equal(42, collection[0].Value);
    }

    [Fact]
    public void SelectDictionary_with_tuple_should_transform_dictionary()
    {
        var source = new Dictionary<int, string> { { 1, "one" }, { 2, "two" } };

        var result = source.SelectDictionary(kv => (kv.Key.ToString(), kv.Value.Length));

        Assert.Equal(2, result.Count);
        Assert.Equal(3, result["1"]);
        Assert.Equal(3, result["2"]);
    }

    [Fact]
    public void SelectDictionary_with_selectors_should_transform_dictionary()
    {
        var source = new Dictionary<int, string> { { 1, "one" }, { 2, "two" } };

        var result = source.SelectDictionary(
            kv => kv.Key * 10,
            kv => kv.Value.ToUpper());

        Assert.Equal(2, result.Count);
        Assert.Equal("ONE", result[10]);
        Assert.Equal("TWO", result[20]);
    }

    [Fact]
    public void SelectDictionary_should_handle_empty_dictionary()
    {
        var source = new Dictionary<int, string>();

        var result = source.SelectDictionary(kv => (kv.Key.ToString(), kv.Value));

        Assert.Empty(result);
    }
}

// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Common.Extensions;

namespace Mouseion.Common.Tests.Extensions;

public class StringExtensionsTests
{
    [Fact]
    public void NullSafe_should_return_string_when_not_null()
    {
        var result = "test".NullSafe();

        Assert.Equal("test", result);
    }

    [Fact]
    public void NullSafe_should_return_null_marker_when_null()
    {
        string? input = null;
        var result = input!.NullSafe();

        Assert.Equal("[NULL]", result);
    }

    [Fact]
    public void FirstCharToLower_should_lowercase_first_character()
    {
        var result = "TestString".FirstCharToLower();

        Assert.Equal("testString", result);
    }

    [Fact]
    public void FirstCharToLower_should_return_empty_for_empty_string()
    {
        var result = "".FirstCharToLower();

        Assert.Equal("", result);
    }

    [Fact]
    public void FirstCharToLower_should_return_empty_for_null()
    {
        string? input = null;
        var result = input!.FirstCharToLower();

        Assert.Equal("", result);
    }

    [Fact]
    public void FirstCharToUpper_should_uppercase_first_character()
    {
        var result = "testString".FirstCharToUpper();

        Assert.Equal("TestString", result);
    }

    [Fact]
    public void FirstCharToUpper_should_return_empty_for_empty_string()
    {
        var result = "".FirstCharToUpper();

        Assert.Equal("", result);
    }

    [Fact]
    public void Inject_should_format_string_with_args()
    {
        var result = "Hello {0}, you are {1}".Inject("World", "awesome");

        Assert.Equal("Hello World, you are awesome", result);
    }

    [Fact]
    public void Replace_should_replace_at_index()
    {
        var result = "Hello World".Replace(6, 5, "Universe");

        Assert.Equal("Hello Universe", result);
    }

    [Fact]
    public void RemoveAccent_should_remove_diacritics()
    {
        var result = "café résumé naïve".RemoveAccent();

        Assert.Equal("cafe resume naive", result);
    }

    [Fact]
    public void TrimEnd_should_remove_postfix()
    {
        var result = "filename.txt".TrimEnd(".txt");

        Assert.Equal("filename", result);
    }

    [Fact]
    public void TrimEnd_should_not_modify_when_postfix_missing()
    {
        var result = "filename.txt".TrimEnd(".csv");

        Assert.Equal("filename.txt", result);
    }

    [Fact]
    public void Join_should_join_strings_with_separator()
    {
        var result = new[] { "a", "b", "c" }.Join(", ");

        Assert.Equal("a, b, c", result);
    }

    [Fact]
    public void CleanSpaces_should_collapse_multiple_spaces()
    {
        var result = "hello    world   test".CleanSpaces();

        Assert.Equal("hello world test", result);
    }

    [Fact]
    public void IsNullOrWhiteSpace_should_return_true_for_empty()
    {
        Assert.True("".IsNullOrWhiteSpace());
        Assert.True("   ".IsNullOrWhiteSpace());
    }

    [Fact]
    public void IsNullOrWhiteSpace_should_return_false_for_content()
    {
        Assert.False("test".IsNullOrWhiteSpace());
    }

    [Fact]
    public void IsNotNullOrWhiteSpace_should_return_true_for_content()
    {
        Assert.True("test".IsNotNullOrWhiteSpace());
    }

    [Fact]
    public void StartsWithIgnoreCase_should_match_case_insensitive()
    {
        Assert.True("TestString".StartsWithIgnoreCase("test"));
        Assert.True("TestString".StartsWithIgnoreCase("TEST"));
    }

    [Fact]
    public void EndsWithIgnoreCase_should_match_case_insensitive()
    {
        Assert.True("TestString".EndsWithIgnoreCase("string"));
        Assert.True("TestString".EndsWithIgnoreCase("STRING"));
    }

    [Fact]
    public void EqualsIgnoreCase_should_match_case_insensitive()
    {
        Assert.True("Test".EqualsIgnoreCase("test"));
        Assert.True("Test".EqualsIgnoreCase("TEST"));
    }

    [Fact]
    public void ContainsIgnoreCase_should_match_case_insensitive()
    {
        Assert.True("TestString".ContainsIgnoreCase("str"));
        Assert.True("TestString".ContainsIgnoreCase("STR"));
    }

    [Fact]
    public void WrapInQuotes_should_wrap_strings_with_spaces()
    {
        var result = "hello world".WrapInQuotes();

        Assert.Equal("\"hello world\"", result);
    }

    [Fact]
    public void WrapInQuotes_should_not_wrap_strings_without_spaces()
    {
        var result = "hello".WrapInQuotes();

        Assert.Equal("hello", result);
    }

    [Fact]
    public void HexToByteArray_should_convert_hex_string()
    {
        var result = "48454C4C4F".HexToByteArray();

        Assert.Equal(new byte[] { 0x48, 0x45, 0x4C, 0x4C, 0x4F }, result);
    }

    [Fact]
    public void ToHexString_should_convert_bytes_to_hex()
    {
        var result = new byte[] { 0x48, 0x45, 0x4C, 0x4C, 0x4F }.ToHexString();

        Assert.Equal("48454C4C4F", result);
    }

    [Fact]
    public void SplitCamelCase_should_insert_spaces()
    {
        var result = "ThisIsCamelCase".SplitCamelCase();

        Assert.Equal("This Is Camel Case", result);
    }

    [Fact]
    public void ContainsIgnoreCase_enumerable_should_match_case_insensitive()
    {
        var list = new[] { "Apple", "Banana", "Cherry" };

        Assert.True(list.ContainsIgnoreCase("apple"));
        Assert.True(list.ContainsIgnoreCase("BANANA"));
    }

    [Fact]
    public void Reverse_should_reverse_string()
    {
        var result = "hello".Reverse();

        Assert.Equal("olleh", result);
    }

    [Fact]
    public void IsValidIpAddress_should_validate_ipv4()
    {
        Assert.True("192.168.1.1".IsValidIpAddress());
        Assert.True("10.0.0.1".IsValidIpAddress());
    }

    [Fact]
    public void IsValidIpAddress_should_reject_broadcast()
    {
        Assert.False("255.255.255.255".IsValidIpAddress());
    }

    [Fact]
    public void IsValidIpAddress_should_reject_invalid()
    {
        Assert.False("not.an.ip".IsValidIpAddress());
        Assert.False("999.999.999.999".IsValidIpAddress());
    }

    [Fact]
    public void ToUrlHost_should_wrap_ipv6_in_brackets()
    {
        var result = "::1".ToUrlHost();

        Assert.Equal("[::1]", result);
    }

    [Fact]
    public void ToUrlHost_should_not_wrap_ipv4()
    {
        var result = "192.168.1.1".ToUrlHost();

        Assert.Equal("192.168.1.1", result);
    }

    [Fact]
    public void SanitizeForLog_should_truncate_long_strings()
    {
        var longString = new string('a', 2000);
        var result = longString.SanitizeForLog(100);

        Assert.Equal(100 + "...[truncated]".Length, result.Length);
        Assert.EndsWith("...[truncated]", result);
    }

    [Fact]
    public void SanitizeForLog_should_replace_control_characters()
    {
        var result = "hello\nworld\ttab".SanitizeForLog();

        Assert.Equal("hello world tab", result);
    }

    [Fact]
    public void SanitizeForLog_should_return_empty_for_null()
    {
        string? input = null;
        var result = input.SanitizeForLog();

        Assert.Equal("", result);
    }

    [Fact]
    public void SafeFilename_should_replace_invalid_characters()
    {
        var result = "file:name<with>invalid|chars".SafeFilename();

        Assert.DoesNotContain(":", result);
        Assert.DoesNotContain("<", result);
        Assert.DoesNotContain(">", result);
        Assert.DoesNotContain("|", result);
    }

    [Fact]
    public void SafeFilename_should_return_empty_for_whitespace()
    {
        var result = "   ".SafeFilename();

        Assert.Equal("", result);
    }

    [Fact]
    public void SafeFilename_should_replace_directory_separators()
    {
        var result = "path/to/file".SafeFilename();

        Assert.DoesNotContain("/", result);
    }
}

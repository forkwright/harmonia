// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Common.Extensions;

namespace Mouseion.Common.Tests.Extensions;

public class TryParseExtensionsTests
{
    [Fact]
    public void ParseInt32_should_parse_valid_integer()
    {
        var result = "42".ParseInt32();

        Assert.Equal(42, result);
    }

    [Fact]
    public void ParseInt32_should_parse_negative_integer()
    {
        var result = "-123".ParseInt32();

        Assert.Equal(-123, result);
    }

    [Fact]
    public void ParseInt32_should_parse_zero()
    {
        var result = "0".ParseInt32();

        Assert.Equal(0, result);
    }

    [Fact]
    public void ParseInt32_should_return_null_for_invalid_input()
    {
        var result = "not a number".ParseInt32();

        Assert.Null(result);
    }

    [Fact]
    public void ParseInt32_should_return_null_for_empty_string()
    {
        var result = "".ParseInt32();

        Assert.Null(result);
    }

    [Fact]
    public void ParseInt32_should_return_null_for_overflow()
    {
        var result = "999999999999999".ParseInt32();

        Assert.Null(result);
    }

    [Fact]
    public void ParseInt64_should_parse_valid_long()
    {
        var result = "9223372036854775807".ParseInt64();

        Assert.Equal(long.MaxValue, result);
    }

    [Fact]
    public void ParseInt64_should_parse_negative_long()
    {
        var result = "-9223372036854775808".ParseInt64();

        Assert.Equal(long.MinValue, result);
    }

    [Fact]
    public void ParseInt64_should_return_null_for_invalid_input()
    {
        var result = "invalid".ParseInt64();

        Assert.Null(result);
    }

    [Fact]
    public void ParseInt64_should_return_null_for_empty_string()
    {
        var result = "".ParseInt64();

        Assert.Null(result);
    }

    [Fact]
    public void ParseDouble_should_parse_valid_double()
    {
        var result = "3.14159".ParseDouble();

        Assert.NotNull(result);
        Assert.Equal(3.14159, result.Value, 5);
    }

    [Fact]
    public void ParseDouble_should_parse_negative_double()
    {
        var result = "-2.5".ParseDouble();

        Assert.NotNull(result);
        Assert.Equal(-2.5, result.Value, 5);
    }

    [Fact]
    public void ParseDouble_should_parse_integer_as_double()
    {
        var result = "42".ParseDouble();

        Assert.NotNull(result);
        Assert.Equal(42.0, result.Value, 5);
    }

    [Fact]
    public void ParseDouble_should_handle_comma_decimal_separator()
    {
        // The implementation replaces comma with period
        var result = "3,14".ParseDouble();

        Assert.NotNull(result);
        Assert.Equal(3.14, result.Value, 5);
    }

    [Fact]
    public void ParseDouble_should_return_null_for_invalid_input()
    {
        var result = "not a double".ParseDouble();

        Assert.Null(result);
    }

    [Fact]
    public void ParseDouble_should_return_null_for_empty_string()
    {
        var result = "".ParseDouble();

        Assert.Null(result);
    }

    [Fact]
    public void ParseDouble_should_parse_scientific_notation()
    {
        var result = "1.5e10".ParseDouble();

        Assert.NotNull(result);
        Assert.Equal(1.5e10, result.Value, 0);
    }

    [Fact]
    public void ParseDouble_should_parse_zero()
    {
        var result = "0.0".ParseDouble();

        Assert.NotNull(result);
        Assert.Equal(0.0, result.Value, 5);
    }
}

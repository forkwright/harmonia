// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Common.Extensions;

namespace Mouseion.Common.Tests.Extensions;

public class NumberExtensionsTests
{
    [Fact]
    public void SizeSuffix_should_return_bytes_for_small_values()
    {
        var result = 500L.SizeSuffix();

        Assert.Equal("500.0 B", result);
    }

    [Fact]
    public void SizeSuffix_should_return_zero_bytes_for_zero()
    {
        var result = 0L.SizeSuffix();

        Assert.Equal("0 B", result);
    }

    [Fact]
    public void SizeSuffix_should_return_kilobytes()
    {
        var result = 1024L.SizeSuffix();

        Assert.Equal("1.0 KB", result);
    }

    [Fact]
    public void SizeSuffix_should_return_megabytes()
    {
        var result = (1024L * 1024).SizeSuffix();

        Assert.Equal("1.0 MB", result);
    }

    [Fact]
    public void SizeSuffix_should_return_gigabytes()
    {
        var result = (1024L * 1024 * 1024).SizeSuffix();

        Assert.Equal("1.0 GB", result);
    }

    [Fact]
    public void SizeSuffix_should_return_terabytes()
    {
        var result = (1024L * 1024 * 1024 * 1024).SizeSuffix();

        Assert.Equal("1.0 TB", result);
    }

    [Fact]
    public void SizeSuffix_should_handle_negative_values()
    {
        var result = (-1024L).SizeSuffix();

        Assert.Equal("-1.0 KB", result);
    }

    [Fact]
    public void SizeSuffix_should_format_decimal_values()
    {
        var result = 1536L.SizeSuffix();

        Assert.Equal("1.5 KB", result);
    }

    [Fact]
    public void Megabytes_int_should_convert_correctly()
    {
        var result = 1.Megabytes();

        Assert.Equal(1024L * 1024, result);
    }

    [Fact]
    public void Megabytes_int_should_handle_large_values()
    {
        var result = 100.Megabytes();

        Assert.Equal(100L * 1024 * 1024, result);
    }

    [Fact]
    public void Gigabytes_int_should_convert_correctly()
    {
        var result = 1.Gigabytes();

        Assert.Equal(1024L * 1024 * 1024, result);
    }

    [Fact]
    public void Gigabytes_int_should_handle_large_values()
    {
        var result = 10.Gigabytes();

        Assert.Equal(10L * 1024 * 1024 * 1024, result);
    }

    [Fact]
    public void Megabytes_double_should_convert_correctly()
    {
        var result = 1.5.Megabytes();

        Assert.Equal((long)(1.5 * 1024 * 1024), result);
    }

    [Fact]
    public void Gigabytes_double_should_convert_correctly()
    {
        var result = 2.5.Gigabytes();

        Assert.Equal((long)(2.5 * 1024 * 1024 * 1024), result);
    }

    [Fact]
    public void Megabytes_zero_should_return_zero()
    {
        var result = 0.Megabytes();

        Assert.Equal(0L, result);
    }

    [Fact]
    public void Gigabytes_zero_should_return_zero()
    {
        var result = 0.Gigabytes();

        Assert.Equal(0L, result);
    }
}

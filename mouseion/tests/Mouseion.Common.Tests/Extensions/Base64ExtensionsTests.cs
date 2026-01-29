// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Common.Extensions;

namespace Mouseion.Common.Tests.Extensions;

public class Base64ExtensionsTests
{
    [Fact]
    public void ToBase64_bytes_should_encode_correctly()
    {
        var bytes = new byte[] { 72, 101, 108, 108, 111 }; // "Hello"

        var result = bytes.ToBase64();

        Assert.Equal("SGVsbG8=", result);
    }

    [Fact]
    public void ToBase64_bytes_should_handle_empty_array()
    {
        var bytes = Array.Empty<byte>();

        var result = bytes.ToBase64();

        Assert.Equal("", result);
    }

    [Fact]
    public void ToBase64_bytes_should_handle_single_byte()
    {
        var bytes = new byte[] { 65 }; // "A"

        var result = bytes.ToBase64();

        Assert.Equal("QQ==", result);
    }

    [Fact]
    public void ToBase64_bytes_should_handle_binary_data()
    {
        var bytes = new byte[] { 0x00, 0xFF, 0x7F, 0x80 };

        var result = bytes.ToBase64();

        Assert.Equal("AP9/gA==", result);
    }

    [Fact]
    public void ToBase64_long_should_encode_zero()
    {
        var result = 0L.ToBase64();

        Assert.Equal("AAAAAAAAAAA=", result);
    }

    [Fact]
    public void ToBase64_long_should_encode_positive_number()
    {
        var result = 12345678901234L.ToBase64();

        // Convert to bytes and then base64
        var expected = Convert.ToBase64String(BitConverter.GetBytes(12345678901234L));

        Assert.Equal(expected, result);
    }

    [Fact]
    public void ToBase64_long_should_encode_max_value()
    {
        var result = long.MaxValue.ToBase64();

        var expected = Convert.ToBase64String(BitConverter.GetBytes(long.MaxValue));

        Assert.Equal(expected, result);
    }

    [Fact]
    public void ToBase64_long_should_encode_min_value()
    {
        var result = long.MinValue.ToBase64();

        var expected = Convert.ToBase64String(BitConverter.GetBytes(long.MinValue));

        Assert.Equal(expected, result);
    }

    [Fact]
    public void ToBase64_long_should_encode_negative_number()
    {
        var result = (-1L).ToBase64();

        var expected = Convert.ToBase64String(BitConverter.GetBytes(-1L));

        Assert.Equal(expected, result);
    }

    [Fact]
    public void ToBase64_roundtrip_should_work()
    {
        var original = new byte[] { 1, 2, 3, 4, 5 };

        var encoded = original.ToBase64();
        var decoded = Convert.FromBase64String(encoded);

        Assert.Equal(original, decoded);
    }
}

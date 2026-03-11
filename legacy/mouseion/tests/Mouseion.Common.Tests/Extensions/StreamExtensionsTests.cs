// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Common.Extensions;

namespace Mouseion.Common.Tests.Extensions;

public class StreamExtensionsTests
{
    [Fact]
    public void ToBytes_should_convert_stream_to_byte_array()
    {
        var data = new byte[] { 1, 2, 3, 4, 5 };
        using var stream = new MemoryStream(data);

        var result = stream.ToBytes();

        Assert.Equal(data, result);
    }

    [Fact]
    public void ToBytes_should_handle_empty_stream()
    {
        using var stream = new MemoryStream();

        var result = stream.ToBytes();

        Assert.Empty(result);
    }

    [Fact]
    public void ToBytes_should_handle_large_stream()
    {
        // Create a stream larger than the internal buffer (16KB)
        var data = new byte[32 * 1024];
        for (var i = 0; i < data.Length; i++)
        {
            data[i] = (byte)(i % 256);
        }
        using var stream = new MemoryStream(data);

        var result = stream.ToBytes();

        Assert.Equal(data, result);
    }

    [Fact]
    public void ToBytes_should_read_from_current_position()
    {
        var data = new byte[] { 1, 2, 3, 4, 5 };
        using var stream = new MemoryStream(data);
        stream.Position = 2; // Skip first 2 bytes

        var result = stream.ToBytes();

        Assert.Equal(new byte[] { 3, 4, 5 }, result);
    }

    [Fact]
    public void ToBytes_should_handle_single_byte()
    {
        using var stream = new MemoryStream(new byte[] { 42 });

        var result = stream.ToBytes();

        Assert.Single(result);
        Assert.Equal(42, result[0]);
    }

    [Fact]
    public void ToBytes_should_work_with_non_seekable_stream()
    {
        // NetworkStream is non-seekable, but we can simulate with a wrapper
        var data = new byte[] { 1, 2, 3, 4, 5 };
        using var baseStream = new MemoryStream(data);
        using var nonSeekableStream = new NonSeekableStreamWrapper(baseStream);

        var result = nonSeekableStream.ToBytes();

        Assert.Equal(data, result);
    }

    private class NonSeekableStreamWrapper : Stream
    {
        private readonly Stream _inner;

        public NonSeekableStreamWrapper(Stream inner) => _inner = inner;

        public override bool CanRead => _inner.CanRead;
        public override bool CanSeek => false;
        public override bool CanWrite => _inner.CanWrite;
        public override long Length => throw new NotSupportedException();
        public override long Position
        {
            get => throw new NotSupportedException();
            set => throw new NotSupportedException();
        }

        public override void Flush() => _inner.Flush();
        public override int Read(byte[] buffer, int offset, int count) => _inner.Read(buffer, offset, count);
        public override long Seek(long offset, SeekOrigin origin) => throw new NotSupportedException();
        public override void SetLength(long value) => throw new NotSupportedException();
        public override void Write(byte[] buffer, int offset, int count) => _inner.Write(buffer, offset, count);
    }
}

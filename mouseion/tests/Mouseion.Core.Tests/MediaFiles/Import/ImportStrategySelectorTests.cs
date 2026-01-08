// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging.Abstractions;
using Moq;
using Mouseion.Common.Disk;
using Mouseion.Core.MediaFiles.Import;
using Mouseion.Core.Movies.Organization;

namespace Mouseion.Core.Tests.MediaFiles.Import;

public class ImportStrategySelectorTests
{
    private readonly Mock<IDiskProvider> _diskProviderMock;
    private readonly ImportStrategySelector _selector;

    public ImportStrategySelectorTests()
    {
        _diskProviderMock = new Mock<IDiskProvider>();
        _selector = new ImportStrategySelector(
            _diskProviderMock.Object,
            NullLogger<ImportStrategySelector>.Instance);
    }

    [Fact]
    public void SelectStrategy_should_respect_user_preference()
    {
        var strategy = _selector.SelectStrategy(
            "/source/file.mkv",
            "/dest/file.mkv",
            FileStrategy.Copy);

        Assert.Equal(FileStrategy.Copy, strategy);
    }

    [Fact]
    public void SelectStrategy_should_prefer_hardlink_on_same_mount()
    {
        var mountMock = new Mock<IMount>();
        mountMock.Setup(x => x.RootDirectory).Returns("/mnt/data");
        mountMock.Setup(x => x.DriveFormat).Returns("ext4");

        _diskProviderMock
            .Setup(x => x.GetMount(It.IsAny<string>()))
            .Returns(mountMock.Object);

        var strategy = _selector.SelectStrategy(
            "/mnt/data/source/file.mkv",
            "/mnt/data/dest/file.mkv");

        Assert.Equal(FileStrategy.Hardlink, strategy);
    }

    [Fact]
    public void SelectStrategy_should_prefer_copy_for_network_filesystem()
    {
        var sourceMountMock = new Mock<IMount>();
        sourceMountMock.Setup(x => x.RootDirectory).Returns("/mnt/network");
        sourceMountMock.Setup(x => x.DriveFormat).Returns("cifs");

        var destMountMock = new Mock<IMount>();
        destMountMock.Setup(x => x.RootDirectory).Returns("/mnt/local");
        destMountMock.Setup(x => x.DriveFormat).Returns("ext4");

        _diskProviderMock
            .Setup(x => x.GetMount("/mnt/network/file.mkv"))
            .Returns(sourceMountMock.Object);

        _diskProviderMock
            .Setup(x => x.GetMount("/mnt/local/file.mkv"))
            .Returns(destMountMock.Object);

        var strategy = _selector.SelectStrategy(
            "/mnt/network/file.mkv",
            "/mnt/local/file.mkv");

        Assert.Equal(FileStrategy.Copy, strategy);
    }

    [Fact]
    public void SelectStrategy_should_prefer_hardlink_for_cow_filesystem()
    {
        var sourceMountMock = new Mock<IMount>();
        sourceMountMock.Setup(x => x.RootDirectory).Returns("/mnt/btrfs");
        sourceMountMock.Setup(x => x.DriveFormat).Returns("btrfs");

        var destMountMock = new Mock<IMount>();
        destMountMock.Setup(x => x.RootDirectory).Returns("/mnt/ext4");
        destMountMock.Setup(x => x.DriveFormat).Returns("ext4");

        _diskProviderMock
            .Setup(x => x.GetMount("/mnt/btrfs/file.mkv"))
            .Returns(sourceMountMock.Object);

        _diskProviderMock
            .Setup(x => x.GetMount("/mnt/ext4/file.mkv"))
            .Returns(destMountMock.Object);

        var strategy = _selector.SelectStrategy(
            "/mnt/btrfs/file.mkv",
            "/mnt/ext4/file.mkv");

        Assert.Equal(FileStrategy.Hardlink, strategy);
    }

    [Fact]
    public void SelectStrategy_should_default_to_hardlink_when_filesystem_unknown()
    {
        var sourceMountMock = new Mock<IMount>();
        sourceMountMock.Setup(x => x.RootDirectory).Returns("/source");
        sourceMountMock.Setup(x => x.DriveFormat).Returns((string?)null);

        var destMountMock = new Mock<IMount>();
        destMountMock.Setup(x => x.RootDirectory).Returns("/dest");
        destMountMock.Setup(x => x.DriveFormat).Returns((string?)null);

        _diskProviderMock
            .Setup(x => x.GetMount("/source/file.mkv"))
            .Returns(sourceMountMock.Object);

        _diskProviderMock
            .Setup(x => x.GetMount("/dest/file.mkv"))
            .Returns(destMountMock.Object);

        var strategy = _selector.SelectStrategy(
            "/source/file.mkv",
            "/dest/file.mkv");

        Assert.Equal(FileStrategy.Hardlink, strategy);
    }

    [Fact]
    public void SelectStrategy_should_handle_null_mount_info()
    {
        _diskProviderMock
            .Setup(x => x.GetMount(It.IsAny<string>()))
            .Returns((IMount?)null);

        var strategy = _selector.SelectStrategy(
            "/source/file.mkv",
            "/dest/file.mkv");

        Assert.Equal(FileStrategy.Hardlink, strategy);
    }

    [Theory]
    [InlineData("cifs")]
    [InlineData("smb")]
    [InlineData("nfs")]
    public void SelectStrategy_should_detect_various_network_filesystems(string driveFormat)
    {
        var networkMountMock = new Mock<IMount>();
        networkMountMock.Setup(x => x.RootDirectory).Returns("/mnt/network");
        networkMountMock.Setup(x => x.DriveFormat).Returns(driveFormat);

        var localMountMock = new Mock<IMount>();
        localMountMock.Setup(x => x.RootDirectory).Returns("/local");
        localMountMock.Setup(x => x.DriveFormat).Returns("ext4");

        _diskProviderMock
            .Setup(x => x.GetMount("/mnt/network/file.mkv"))
            .Returns(networkMountMock.Object);

        _diskProviderMock
            .Setup(x => x.GetMount("/local/file.mkv"))
            .Returns(localMountMock.Object);

        var strategy = _selector.SelectStrategy(
            "/mnt/network/file.mkv",
            "/local/file.mkv");

        Assert.Equal(FileStrategy.Copy, strategy);
    }

    [Theory]
    [InlineData("btrfs")]
    [InlineData("zfs")]
    [InlineData("apfs")]
    public void SelectStrategy_should_detect_various_cow_filesystems(string driveFormat)
    {
        var cowMountMock = new Mock<IMount>();
        cowMountMock.Setup(x => x.RootDirectory).Returns("/mnt/cow");
        cowMountMock.Setup(x => x.DriveFormat).Returns(driveFormat);

        var otherMountMock = new Mock<IMount>();
        otherMountMock.Setup(x => x.RootDirectory).Returns("/other");
        otherMountMock.Setup(x => x.DriveFormat).Returns("ext4");

        _diskProviderMock
            .Setup(x => x.GetMount("/mnt/cow/file.mkv"))
            .Returns(cowMountMock.Object);

        _diskProviderMock
            .Setup(x => x.GetMount("/other/file.mkv"))
            .Returns(otherMountMock.Object);

        var strategy = _selector.SelectStrategy(
            "/mnt/cow/file.mkv",
            "/other/file.mkv");

        Assert.Equal(FileStrategy.Hardlink, strategy);
    }
}

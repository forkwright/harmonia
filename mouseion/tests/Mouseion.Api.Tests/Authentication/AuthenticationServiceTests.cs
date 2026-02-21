// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentAssertions;
using Moq;
using Mouseion.Core.Authentication;
using Xunit;

namespace Mouseion.Api.Tests.Authentication;

public class AuthenticationServiceTests
{
    private readonly Mock<IUserRepository> _userRepo;
    private readonly AuthenticationService _sut;

    public AuthenticationServiceTests()
    {
        _userRepo = new Mock<IUserRepository>();
        _sut = new AuthenticationService(_userRepo.Object);
    }

    [Fact]
    public void HashPassword_ProducesVerifiableHash()
    {
        var password = "TestPassword123!";
        var hash = _sut.HashPassword(password);

        hash.Should().NotBeNullOrEmpty();
        hash.Should().Contain("."); // Format: iterations.algorithm.salt.hash
        _sut.VerifyPassword(password, hash).Should().BeTrue();
    }

    [Fact]
    public void HashPassword_DifferentSaltEachTime()
    {
        var password = "TestPassword123!";
        var hash1 = _sut.HashPassword(password);
        var hash2 = _sut.HashPassword(password);

        hash1.Should().NotBe(hash2);
        _sut.VerifyPassword(password, hash1).Should().BeTrue();
        _sut.VerifyPassword(password, hash2).Should().BeTrue();
    }

    [Fact]
    public void VerifyPassword_WrongPassword_ReturnsFalse()
    {
        var hash = _sut.HashPassword("CorrectPassword");
        _sut.VerifyPassword("WrongPassword", hash).Should().BeFalse();
    }

    [Fact]
    public void VerifyPassword_MalformedHash_ReturnsFalse()
    {
        _sut.VerifyPassword("any", "not-a-valid-hash").Should().BeFalse();
    }

    [Fact]
    public async Task ValidateCredentials_ValidUser_ReturnsUser()
    {
        var password = "TestPassword123!";
        var user = new User
        {
            Id = 1,
            Username = "testuser",
            PasswordHash = _sut.HashPassword(password),
            IsActive = true,
            AuthenticationMethod = "local"
        };

        _userRepo.Setup(r => r.GetByUsernameAsync("testuser", It.IsAny<CancellationToken>()))
            .ReturnsAsync(user);

        var result = await _sut.ValidateCredentialsAsync("testuser", password);

        result.Should().NotBeNull();
        result!.Username.Should().Be("testuser");
        _userRepo.Verify(r => r.UpdateLastLoginAsync(1, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task ValidateCredentials_WrongPassword_ReturnsNull()
    {
        var user = new User
        {
            Id = 1,
            Username = "testuser",
            PasswordHash = _sut.HashPassword("CorrectPassword"),
            IsActive = true,
            AuthenticationMethod = "local"
        };

        _userRepo.Setup(r => r.GetByUsernameAsync("testuser", It.IsAny<CancellationToken>()))
            .ReturnsAsync(user);

        var result = await _sut.ValidateCredentialsAsync("testuser", "WrongPassword");

        result.Should().BeNull();
    }

    [Fact]
    public async Task ValidateCredentials_InactiveUser_ReturnsNull()
    {
        var user = new User
        {
            Id = 1,
            Username = "testuser",
            PasswordHash = _sut.HashPassword("password"),
            IsActive = false,
            AuthenticationMethod = "local"
        };

        _userRepo.Setup(r => r.GetByUsernameAsync("testuser", It.IsAny<CancellationToken>()))
            .ReturnsAsync(user);

        var result = await _sut.ValidateCredentialsAsync("testuser", "password");

        result.Should().BeNull();
    }

    [Fact]
    public async Task ValidateCredentials_NonLocalAuth_ReturnsNull()
    {
        var user = new User
        {
            Id = 1,
            Username = "testuser",
            IsActive = true,
            AuthenticationMethod = "oidc"
        };

        _userRepo.Setup(r => r.GetByUsernameAsync("testuser", It.IsAny<CancellationToken>()))
            .ReturnsAsync(user);

        var result = await _sut.ValidateCredentialsAsync("testuser", "password");

        result.Should().BeNull();
    }

    [Fact]
    public async Task ValidateCredentials_UnknownUser_ReturnsNull()
    {
        _userRepo.Setup(r => r.GetByUsernameAsync("nobody", It.IsAny<CancellationToken>()))
            .ReturnsAsync((User?)null);

        var result = await _sut.ValidateCredentialsAsync("nobody", "password");

        result.Should().BeNull();
    }

    [Fact]
    public async Task CreateUser_DuplicateUsername_Throws()
    {
        _userRepo.Setup(r => r.UsernameExistsAsync("taken", It.IsAny<CancellationToken>()))
            .ReturnsAsync(true);

        var act = async () => await _sut.CreateUserAsync(new CreateUserRequest
        {
            Username = "taken",
            Password = "password123"
        });

        await act.Should().ThrowAsync<InvalidOperationException>()
            .WithMessage("*already taken*");
    }

    [Fact]
    public async Task CreateUser_DuplicateEmail_Throws()
    {
        _userRepo.Setup(r => r.UsernameExistsAsync(It.IsAny<string>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(false);
        _userRepo.Setup(r => r.EmailExistsAsync("taken@test.com", It.IsAny<CancellationToken>()))
            .ReturnsAsync(true);

        var act = async () => await _sut.CreateUserAsync(new CreateUserRequest
        {
            Username = "newuser",
            Password = "password123",
            Email = "taken@test.com"
        });

        await act.Should().ThrowAsync<InvalidOperationException>()
            .WithMessage("*already registered*");
    }

    [Fact]
    public async Task CreateUser_Valid_ReturnsCreatedUser()
    {
        _userRepo.Setup(r => r.UsernameExistsAsync(It.IsAny<string>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(false);
        _userRepo.Setup(r => r.InsertAsync(It.IsAny<User>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((User u, CancellationToken _) => { u.Id = 1; return u; });

        var result = await _sut.CreateUserAsync(new CreateUserRequest
        {
            Username = "newuser",
            Password = "password123",
            DisplayName = "New User",
            Role = UserRole.User
        });

        result.Username.Should().Be("newuser");
        result.DisplayName.Should().Be("New User");
        result.Role.Should().Be(UserRole.User);
        result.PasswordHash.Should().NotBeEmpty();
        result.AuthenticationMethod.Should().Be("local");
    }

    [Fact]
    public async Task ChangePassword_WrongCurrent_Throws()
    {
        var user = new User
        {
            Id = 1,
            PasswordHash = _sut.HashPassword("current")
        };

        _userRepo.Setup(r => r.GetAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(user);

        var act = async () => await _sut.ChangePasswordAsync(1, "wrong", "newpass");

        await act.Should().ThrowAsync<UnauthorizedAccessException>();
    }

    [Fact]
    public async Task ChangePassword_CorrectCurrent_UpdatesHash()
    {
        var user = new User
        {
            Id = 1,
            PasswordHash = _sut.HashPassword("current")
        };

        _userRepo.Setup(r => r.GetAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(user);
        _userRepo.Setup(r => r.UpdateAsync(It.IsAny<User>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((User u, CancellationToken _) => u);

        await _sut.ChangePasswordAsync(1, "current", "newpassword");

        _userRepo.Verify(r => r.UpdateAsync(
            It.Is<User>(u => _sut.VerifyPassword("newpassword", u.PasswordHash)),
            It.IsAny<CancellationToken>()), Times.Once);
    }
}

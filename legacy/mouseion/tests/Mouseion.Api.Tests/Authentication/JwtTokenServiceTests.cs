// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Security.Claims;
using FluentAssertions;
using Moq;
using Mouseion.Core.Authentication;
using Xunit;

namespace Mouseion.Api.Tests.Authentication;

public class JwtTokenServiceTests
{
    private readonly Mock<IRefreshTokenRepository> _refreshTokenRepo;
    private readonly JwtSettings _settings;
    private readonly JwtTokenService _sut;

    public JwtTokenServiceTests()
    {
        _refreshTokenRepo = new Mock<IRefreshTokenRepository>();
        _settings = new JwtSettings
        {
            SecretKey = "this-is-a-test-secret-key-that-must-be-at-least-32-characters-long",
            Issuer = "mouseion-test",
            Audience = "mouseion-test-clients",
            AccessTokenExpiry = TimeSpan.FromMinutes(15),
            RefreshTokenExpiry = TimeSpan.FromDays(30)
        };
        _sut = new JwtTokenService(_refreshTokenRepo.Object, _settings);
    }

    [Fact]
    public void GenerateAccessToken_ContainsExpectedClaims()
    {
        var user = new User
        {
            Id = 42,
            Username = "testuser",
            DisplayName = "Test User",
            Email = "test@example.com",
            Role = UserRole.Admin
        };

        var token = _sut.GenerateAccessToken(user);

        token.Should().NotBeNullOrEmpty();

        var principal = _sut.ValidateAccessToken(token);
        principal.Should().NotBeNull();
        principal!.FindFirst(ClaimTypes.NameIdentifier)?.Value.Should().Be("42");
        principal.FindFirst(ClaimTypes.Name)?.Value.Should().Be("testuser");
        principal.FindFirst(ClaimTypes.Role)?.Value.Should().Be("Admin");
        principal.FindFirst("display_name")?.Value.Should().Be("Test User");
        principal.FindFirst(ClaimTypes.Email)?.Value.Should().Be("test@example.com");
    }

    [Fact]
    public void ValidateAccessToken_InvalidToken_ReturnsNull()
    {
        _sut.ValidateAccessToken("not-a-valid-jwt").Should().BeNull();
    }

    [Fact]
    public void ValidateAccessToken_ExpiredToken_ReturnsNull()
    {
        // Create a settings with negative expiry to produce an already-expired token
        var expiredSettings = new JwtSettings
        {
            SecretKey = _settings.SecretKey,
            Issuer = _settings.Issuer,
            Audience = _settings.Audience,
            AccessTokenExpiry = TimeSpan.FromSeconds(-1)
        };
        var expiredService = new JwtTokenService(_refreshTokenRepo.Object, expiredSettings);

        var user = new User { Id = 1, Username = "test", DisplayName = "Test" };
        var token = expiredService.GenerateAccessToken(user);

        // With 30s clock skew, a -1s token might still be valid, so use a wider margin
        var wideExpiredSettings = new JwtSettings
        {
            SecretKey = _settings.SecretKey,
            Issuer = _settings.Issuer,
            Audience = _settings.Audience,
            AccessTokenExpiry = TimeSpan.FromMinutes(-2)
        };
        var wideExpiredService = new JwtTokenService(_refreshTokenRepo.Object, wideExpiredSettings);
        var expiredToken = wideExpiredService.GenerateAccessToken(user);

        _sut.ValidateAccessToken(expiredToken).Should().BeNull();
    }

    [Fact]
    public async Task GenerateRefreshToken_StoresInRepository()
    {
        _refreshTokenRepo.Setup(r => r.InsertAsync(It.IsAny<RefreshToken>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((RefreshToken t, CancellationToken _) => { t.Id = 1; return t; });

        var token = await _sut.GenerateRefreshTokenAsync(42, "iPhone");

        token.Should().NotBeNullOrEmpty();

        _refreshTokenRepo.Verify(r => r.InsertAsync(
            It.Is<RefreshToken>(t =>
                t.UserId == 42 &&
                t.DeviceName == "iPhone" &&
                t.Token == token &&
                t.ExpiresAt > DateTime.UtcNow),
            It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task RefreshTokens_ValidToken_RotatesAndReturnsNew()
    {
        var existingToken = new RefreshToken
        {
            Id = 1,
            UserId = 42,
            Token = "old-token",
            ExpiresAt = DateTime.UtcNow.AddDays(1),
            CreatedAt = DateTime.UtcNow.AddHours(-1)
        };

        _refreshTokenRepo.Setup(r => r.GetByTokenAsync("old-token", It.IsAny<CancellationToken>()))
            .ReturnsAsync(existingToken);
        _refreshTokenRepo.Setup(r => r.InsertAsync(It.IsAny<RefreshToken>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((RefreshToken t, CancellationToken _) => { t.Id = 2; return t; });

        var result = await _sut.RefreshTokensAsync("old-token", "browser");

        result.UserId.Should().Be(42);
        result.RefreshToken.Should().NotBe("old-token");
        _refreshTokenRepo.Verify(r => r.RevokeTokenAsync("old-token", It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task RefreshTokens_RevokedToken_Throws()
    {
        var revokedToken = new RefreshToken
        {
            Id = 1,
            UserId = 42,
            Token = "revoked-token",
            ExpiresAt = DateTime.UtcNow.AddDays(1),
            RevokedAt = DateTime.UtcNow.AddHours(-1)
        };

        _refreshTokenRepo.Setup(r => r.GetByTokenAsync("revoked-token", It.IsAny<CancellationToken>()))
            .ReturnsAsync(revokedToken);

        var act = async () => await _sut.RefreshTokensAsync("revoked-token", "browser");

        await act.Should().ThrowAsync<UnauthorizedAccessException>();
    }

    [Fact]
    public async Task RefreshTokens_ExpiredToken_Throws()
    {
        var expiredToken = new RefreshToken
        {
            Id = 1,
            UserId = 42,
            Token = "expired-token",
            ExpiresAt = DateTime.UtcNow.AddDays(-1),
            CreatedAt = DateTime.UtcNow.AddDays(-31)
        };

        _refreshTokenRepo.Setup(r => r.GetByTokenAsync("expired-token", It.IsAny<CancellationToken>()))
            .ReturnsAsync(expiredToken);

        var act = async () => await _sut.RefreshTokensAsync("expired-token", "browser");

        await act.Should().ThrowAsync<UnauthorizedAccessException>();
    }

    [Fact]
    public async Task RefreshTokens_UnknownToken_Throws()
    {
        _refreshTokenRepo.Setup(r => r.GetByTokenAsync("unknown", It.IsAny<CancellationToken>()))
            .ReturnsAsync((RefreshToken?)null);

        var act = async () => await _sut.RefreshTokensAsync("unknown", "browser");

        await act.Should().ThrowAsync<UnauthorizedAccessException>();
    }
}

// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.IdentityModel.Tokens.Jwt;
using System.Security.Claims;
using System.Security.Cryptography;
using System.Text;
using Microsoft.IdentityModel.Tokens;
using Serilog;

namespace Mouseion.Core.Authentication;

public interface IJwtTokenService
{
    string GenerateAccessToken(User user);
    Task<string> GenerateRefreshTokenAsync(int userId, string deviceName, CancellationToken ct = default);
    Task<TokenPair> RefreshTokensAsync(string refreshToken, string deviceName, CancellationToken ct = default);
    Task RevokeRefreshTokenAsync(string refreshToken, CancellationToken ct = default);
    Task RevokeAllTokensForUserAsync(int userId, CancellationToken ct = default);
    ClaimsPrincipal? ValidateAccessToken(string token);
}

public class JwtTokenService : IJwtTokenService
{
    private readonly IRefreshTokenRepository _refreshTokenRepository;
    private readonly JwtSettings _settings;
    private readonly SymmetricSecurityKey _signingKey;
    private static readonly ILogger Logger = Log.ForContext<JwtTokenService>();

    public JwtTokenService(IRefreshTokenRepository refreshTokenRepository, JwtSettings settings)
    {
        _refreshTokenRepository = refreshTokenRepository;
        _settings = settings;
        _signingKey = new SymmetricSecurityKey(Encoding.UTF8.GetBytes(_settings.SecretKey));
    }

    public string GenerateAccessToken(User user)
    {
        var claims = new List<Claim>
        {
            new(ClaimTypes.NameIdentifier, user.Id.ToString()),
            new(ClaimTypes.Name, user.Username),
            new(ClaimTypes.Role, user.Role.ToString()),
            new("display_name", user.DisplayName),
            new(JwtRegisteredClaimNames.Jti, Guid.NewGuid().ToString()),
            new(JwtRegisteredClaimNames.Iat, DateTimeOffset.UtcNow.ToUnixTimeSeconds().ToString(), ClaimValueTypes.Integer64)
        };

        if (!string.IsNullOrEmpty(user.Email))
        {
            claims.Add(new Claim(ClaimTypes.Email, user.Email));
        }

        var credentials = new SigningCredentials(_signingKey, SecurityAlgorithms.HmacSha256);

        var token = new JwtSecurityToken(
            issuer: _settings.Issuer,
            audience: _settings.Audience,
            claims: claims,
            expires: DateTime.UtcNow.Add(_settings.AccessTokenExpiry),
            signingCredentials: credentials);

        return new JwtSecurityTokenHandler().WriteToken(token);
    }

    public async Task<string> GenerateRefreshTokenAsync(int userId, string deviceName, CancellationToken ct = default)
    {
        var tokenValue = Convert.ToBase64String(RandomNumberGenerator.GetBytes(64));

        var refreshToken = new RefreshToken
        {
            UserId = userId,
            Token = tokenValue,
            DeviceName = deviceName,
            ExpiresAt = DateTime.UtcNow.Add(_settings.RefreshTokenExpiry),
            CreatedAt = DateTime.UtcNow
        };

        await _refreshTokenRepository.InsertAsync(refreshToken, ct).ConfigureAwait(false);

        return tokenValue;
    }

    public async Task<TokenPair> RefreshTokensAsync(string refreshToken, string deviceName, CancellationToken ct = default)
    {
        var storedToken = await _refreshTokenRepository.GetByTokenAsync(refreshToken, ct).ConfigureAwait(false);

        if (storedToken == null || !storedToken.IsActive)
        {
            throw new UnauthorizedAccessException("Invalid or expired refresh token");
        }

        // Revoke the old refresh token (rotation)
        await _refreshTokenRepository.RevokeTokenAsync(refreshToken, ct).ConfigureAwait(false);

        // Issue new refresh token
        var newRefreshToken = await GenerateRefreshTokenAsync(storedToken.UserId, deviceName, ct).ConfigureAwait(false);

        return new TokenPair
        {
            UserId = storedToken.UserId,
            RefreshToken = newRefreshToken
        };
    }

    public async Task RevokeRefreshTokenAsync(string refreshToken, CancellationToken ct = default)
    {
        await _refreshTokenRepository.RevokeTokenAsync(refreshToken, ct).ConfigureAwait(false);
        Logger.Information("Refresh token revoked");
    }

    public async Task RevokeAllTokensForUserAsync(int userId, CancellationToken ct = default)
    {
        await _refreshTokenRepository.RevokeAllForUserAsync(userId, ct).ConfigureAwait(false);
        Logger.Information("All refresh tokens revoked for user {UserId}", userId);
    }

    public ClaimsPrincipal? ValidateAccessToken(string token)
    {
        try
        {
            var handler = new JwtSecurityTokenHandler();
            var parameters = new TokenValidationParameters
            {
                ValidateIssuer = true,
                ValidateAudience = true,
                ValidateLifetime = true,
                ValidateIssuerSigningKey = true,
                ValidIssuer = _settings.Issuer,
                ValidAudience = _settings.Audience,
                IssuerSigningKey = _signingKey,
                ClockSkew = TimeSpan.FromSeconds(30)
            };

            return handler.ValidateToken(token, parameters, out _);
        }
        catch
        {
            return null;
        }
    }
}

public class JwtSettings
{
    public string SecretKey { get; set; } = string.Empty;
    public string Issuer { get; set; } = "mouseion";
    public string Audience { get; set; } = "mouseion-clients";
    public TimeSpan AccessTokenExpiry { get; set; } = TimeSpan.FromMinutes(15);
    public TimeSpan RefreshTokenExpiry { get; set; } = TimeSpan.FromDays(30);
}

public class TokenPair
{
    public int UserId { get; set; }
    public string AccessToken { get; set; } = string.Empty;
    public string RefreshToken { get; set; } = string.Empty;
}

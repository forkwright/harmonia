// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using System.Security.Claims;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Authentication;

namespace Mouseion.Api.Authentication;

[ApiController]
[Route("api/v3/auth")]
public class AuthController : ControllerBase
{
    private readonly IAuthenticationService _authService;
    private readonly IJwtTokenService _jwtTokenService;
    private readonly IUserRepository _userRepository;

    public AuthController(
        IAuthenticationService authService,
        IJwtTokenService jwtTokenService,
        IUserRepository userRepository)
    {
        _authService = authService;
        _jwtTokenService = jwtTokenService;
        _userRepository = userRepository;
    }

    [HttpPost("login")]
    [AllowAnonymous]
    public async Task<ActionResult<LoginResponse>> Login(
        [FromBody][Required] LoginRequest request,
        CancellationToken ct = default)
    {
        var user = await _authService.ValidateCredentialsAsync(request.Username, request.Password, ct)
            .ConfigureAwait(false);

        if (user == null)
        {
            return Unauthorized(new { error = "Invalid username or password" });
        }

        var accessToken = _jwtTokenService.GenerateAccessToken(user);
        var refreshToken = await _jwtTokenService.GenerateRefreshTokenAsync(
            user.Id, request.DeviceName ?? "unknown", ct).ConfigureAwait(false);

        return Ok(new LoginResponse
        {
            AccessToken = accessToken,
            RefreshToken = refreshToken,
            User = ToUserResource(user)
        });
    }

    [HttpPost("refresh")]
    [AllowAnonymous]
    public async Task<ActionResult<LoginResponse>> Refresh(
        [FromBody][Required] RefreshRequest request,
        CancellationToken ct = default)
    {
        try
        {
            var tokenPair = await _jwtTokenService.RefreshTokensAsync(
                request.RefreshToken, request.DeviceName ?? "unknown", ct).ConfigureAwait(false);

            var user = await _userRepository.GetAsync(tokenPair.UserId, ct).ConfigureAwait(false);
            if (user == null || !user.IsActive)
            {
                return Unauthorized(new { error = "User account is inactive" });
            }

            var accessToken = _jwtTokenService.GenerateAccessToken(user);

            return Ok(new LoginResponse
            {
                AccessToken = accessToken,
                RefreshToken = tokenPair.RefreshToken,
                User = ToUserResource(user)
            });
        }
        catch (UnauthorizedAccessException)
        {
            return Unauthorized(new { error = "Invalid or expired refresh token" });
        }
    }

    [HttpPost("logout")]
    [Authorize]
    public async Task<ActionResult> Logout(
        [FromBody] LogoutRequest? request = null,
        CancellationToken ct = default)
    {
        if (!string.IsNullOrEmpty(request?.RefreshToken))
        {
            await _jwtTokenService.RevokeRefreshTokenAsync(request.RefreshToken, ct).ConfigureAwait(false);
        }
        else
        {
            // Revoke all tokens for this user
            var userId = GetCurrentUserId();
            if (userId.HasValue)
            {
                await _jwtTokenService.RevokeAllTokensForUserAsync(userId.Value, ct).ConfigureAwait(false);
            }
        }

        return NoContent();
    }

    [HttpPost("change-password")]
    [Authorize]
    public async Task<ActionResult> ChangePassword(
        [FromBody][Required] ChangePasswordRequest request,
        CancellationToken ct = default)
    {
        var userId = GetCurrentUserId();
        if (!userId.HasValue)
        {
            return Unauthorized();
        }

        try
        {
            await _authService.ChangePasswordAsync(userId.Value, request.CurrentPassword, request.NewPassword, ct)
                .ConfigureAwait(false);

            // Revoke all existing refresh tokens (force re-login on other devices)
            await _jwtTokenService.RevokeAllTokensForUserAsync(userId.Value, ct).ConfigureAwait(false);

            return NoContent();
        }
        catch (UnauthorizedAccessException)
        {
            return BadRequest(new { error = "Current password is incorrect" });
        }
    }

    private int? GetCurrentUserId()
    {
        var claim = User.FindFirst(ClaimTypes.NameIdentifier);
        return claim != null ? int.Parse(claim.Value) : null;
    }

    internal static UserResource ToUserResource(User user)
    {
        return new UserResource
        {
            Id = user.Id,
            Username = user.Username,
            DisplayName = user.DisplayName,
            Email = user.Email,
            Role = user.Role.ToString(),
            AuthenticationMethod = user.AuthenticationMethod,
            IsActive = user.IsActive,
            CreatedAt = user.CreatedAt,
            LastLoginAt = user.LastLoginAt
        };
    }
}

// Request/Response DTOs
public class LoginRequest
{
    public string Username { get; set; } = string.Empty;
    public string Password { get; set; } = string.Empty;
    public string? DeviceName { get; set; }
}

public class RefreshRequest
{
    public string RefreshToken { get; set; } = string.Empty;
    public string? DeviceName { get; set; }
}

public class LogoutRequest
{
    public string? RefreshToken { get; set; }
}

public class ChangePasswordRequest
{
    public string CurrentPassword { get; set; } = string.Empty;
    public string NewPassword { get; set; } = string.Empty;
}

public class LoginResponse
{
    public string AccessToken { get; set; } = string.Empty;
    public string RefreshToken { get; set; } = string.Empty;
    public UserResource User { get; set; } = null!;
}

public class UserResource
{
    public int Id { get; set; }
    public string Username { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public string Email { get; set; } = string.Empty;
    public string Role { get; set; } = string.Empty;
    public string AuthenticationMethod { get; set; } = string.Empty;
    public bool IsActive { get; set; }
    public DateTime CreatedAt { get; set; }
    public DateTime? LastLoginAt { get; set; }
}

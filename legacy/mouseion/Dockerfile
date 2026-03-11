# syntax=docker/dockerfile:1

# ============================================================================
# Stage 1: Build
# ============================================================================
FROM mcr.microsoft.com/dotnet/sdk:10.0-preview AS build
WORKDIR /src

# Copy solution and project files first for layer caching
COPY Mouseion.sln ./
COPY src/Mouseion.Host/Mouseion.Host.csproj src/Mouseion.Host/
COPY src/Mouseion.Api/Mouseion.Api.csproj src/Mouseion.Api/
COPY src/Mouseion.Core/Mouseion.Core.csproj src/Mouseion.Core/
COPY src/Mouseion.Common/Mouseion.Common.csproj src/Mouseion.Common/
COPY src/Mouseion.SignalR/Mouseion.SignalR.csproj src/Mouseion.SignalR/
COPY tests/Mouseion.Api.Tests/Mouseion.Api.Tests.csproj tests/Mouseion.Api.Tests/
COPY tests/Mouseion.Common.Tests/Mouseion.Common.Tests.csproj tests/Mouseion.Common.Tests/
COPY tests/Mouseion.Core.Tests/Mouseion.Core.Tests.csproj tests/Mouseion.Core.Tests/

# Restore dependencies (cached unless csproj files change)
RUN dotnet restore Mouseion.sln

# Copy everything else and publish
COPY . .
RUN dotnet publish src/Mouseion.Host/Mouseion.Host.csproj \
    --configuration Release \
    --self-contained \
    --runtime linux-x64 \
    --output /app \
    /p:PublishSingleFile=false

# ============================================================================
# Stage 2: Runtime
# ============================================================================
FROM mcr.microsoft.com/dotnet/runtime-deps:10.0-preview-noble AS runtime

# Labels for container registries
LABEL org.opencontainers.image.title="Mouseion" \
      org.opencontainers.image.description="Unified media manager" \
      org.opencontainers.image.source="https://github.com/forkwright/mouseion" \
      org.opencontainers.image.licenses="GPL-3.0-or-later"

# Install SQLite native libs (required by Dapper/FluentMigrator)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libsqlite3-0 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user matching NAS media ownership
# Default UID:GID = 1028:100 (overridable via docker-compose user: directive)
RUN groupadd -g 100 media || true && \
    useradd -u 1028 -g 100 -m -s /bin/bash mouseion

WORKDIR /app
COPY --from=build /app .

# Config volume: DB, logs, JWT secret, MediaCover cache
# Media volumes: mounted read-only from NAS
VOLUME ["/config"]

# Default port
ENV ASPNETCORE_URLS=http://+:8787 \
    ASPNETCORE_ENVIRONMENT=Production \
    ASPNETCORE_FORWARDEDHEADERS_ENABLED=true \
    DOTNET_RUNNING_IN_CONTAINER=true \
    MOUSEION_DATA=/config

EXPOSE 8787

# Health check — /ping endpoint, no auth required
HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD curl -sf http://localhost:8787/ping || exit 1

USER mouseion

ENTRYPOINT ["./Mouseion.Host", "--data=/config"]

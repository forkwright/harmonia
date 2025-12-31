# Multi-stage build for Mouseion
FROM mcr.microsoft.com/dotnet/sdk:10.0 AS build
WORKDIR /src

# Copy solution and project files
COPY Mouseion.sln ./
COPY src/Mouseion.Common/Mouseion.Common.csproj src/Mouseion.Common/
COPY src/Mouseion.Core/Mouseion.Core.csproj src/Mouseion.Core/
COPY src/Mouseion.Api/Mouseion.Api.csproj src/Mouseion.Api/
COPY src/Mouseion.SignalR/Mouseion.SignalR.csproj src/Mouseion.SignalR/
COPY src/Mouseion.Host/Mouseion.Host.csproj src/Mouseion.Host/

# Restore dependencies
RUN dotnet restore

# Copy source code and build
COPY src/ src/
RUN dotnet build Mouseion.sln -c Release --no-restore

# Publish
RUN dotnet publish src/Mouseion.Host/Mouseion.Host.csproj -c Release -o /app/publish --no-restore

# Runtime stage
FROM mcr.microsoft.com/dotnet/aspnet:10.0
WORKDIR /app

# Create non-root user (handle existing GID)
RUN (groupadd -r mouseion --gid=1000 2>/dev/null || groupadd -r mouseion) && \
    (useradd -r -g mouseion --uid=1000 --home-dir=/app \
        --shell=/bin/bash mouseion 2>/dev/null || \
        useradd -r -g mouseion --home-dir=/app --shell=/bin/bash mouseion) && \
    mkdir -p /config && \
    chown -R mouseion:mouseion /app /config

# Copy published app
COPY --from=build /app/publish .

# Set environment
ENV ASPNETCORE_URLS=http://+:7878 \
    MOUSEION_DATA_FOLDER=/config

USER mouseion

EXPOSE 7878

HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:7878/ping || exit 1

VOLUME ["/config"]

ENTRYPOINT ["dotnet", "Mouseion.Host.dll"]

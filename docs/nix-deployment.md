# NixOS Deployment

Harmonia ships a NixOS module for declarative deployment. Add the flake input and import the module to manage the service through `configuration.nix`.

## Flake Input

```nix
# flake.nix
inputs.harmonia.url = "github:forkwright/harmonia";
```

## Minimal Configuration

```nix
# configuration.nix
{ inputs, config, ... }:
{
  imports = [ inputs.harmonia.nixosModules.default ];

  services.harmonia = {
    enable = true;
    settings.paroche.port = 8096;
  };
}
```

## Full Configuration with Secrets

```nix
# configuration.nix
{ inputs, config, ... }:
{
  imports = [ inputs.harmonia.nixosModules.default ];

  services.harmonia = {
    enable = true;
    openFirewall = true;

    # agenix-managed secrets file containing jwt_secret and API keys
    secretsFile = config.age.secrets.harmonia-secrets.path;

    settings = {
      paroche.port = 8096;

      taxis.libraries = {
        music = {
          path = "/media/music";
          media_type = "music";
          watcher_mode = "auto";
        };
        audiobooks = {
          path = "/media/audiobooks";
          media_type = "audiobook";
          watcher_mode = "poll";
          poll_interval_seconds = 300;
        };
        books = {
          path = "/media/books";
          media_type = "book";
          watcher_mode = "poll";
        };
      };

      epignosis.musicbrainz_user_agent =
        "Harmonia/0.1.0 (https://github.com/forkwright/harmonia)";
    };
  };
}
```

## Module Options

| Option | Type | Default | Description |
|---|---|---|---|
| `enable` | bool | `false` | Enable the service |
| `package` | package | `pkgs.harmonia` | Harmonia package (from overlay) |
| `user` | str | `"harmonia"` | System user |
| `group` | str | `"harmonia"` | System group |
| `dataDir` | path | `"/var/lib/harmonia"` | Database and cache directory |
| `settings` | attrs | `{}` | Config written to `harmonia.toml` |
| `secretsFile` | path or null | `null` | Secrets file loaded via `LoadCredential` |
| `openFirewall` | bool | `false` | Open `paroche.port` in the firewall |

## Secret Management

Pass `secretsFile` a path managed by [agenix](https://github.com/ryantm/agenix) or [sops-nix](https://github.com/Mic92/sops-nix). The file is delivered to the service via systemd `LoadCredential`, so it is never world-readable and the path in the environment variable (`HARMONIA_SECRETS_PATH`) points to the credential directory, not the original path.

## Systemd Hardening

The service runs with a hardened systemd profile:

- `NoNewPrivileges`, `PrivateTmp`, `PrivateDevices`
- `ProtectSystem = strict` with explicit `ReadWritePaths` for `dataDir` and library paths
- `MemoryDenyWriteExecute`, `RestrictNamespaces`, `RestrictRealtime`
- `SystemCallFilter = @system-service ~@privileged`

## Overlay

To use the package in your own NixOS config without the module:

```nix
nixpkgs.overlays = [ inputs.harmonia.overlays.default ];
environment.systemPackages = [ pkgs.harmonia ];
```

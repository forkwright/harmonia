# NixOS deployment

Harmonia ships a NixOS module for declarative deployment. Add the flake input and import the module to manage the service through `configuration.nix`.

## Flake input

```nix
# flake.nix
inputs.harmonia.url = "github:forkwright/harmonia";
```

## Minimal configuration

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

## Full configuration with secrets

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

      # horismos::MediaType (crates/horismos/src/subsystems.rs) has only
      # three variants — music | video | book. There is no dedicated
      # "audiobook" or "movie" value at the library-config level; audiobooks
      # and books share `media_type = "book"` here (two libraries, same
      # type, different paths — this is a supported shape). Every field
      # below is required: LibraryConfig has no per-field defaults, so an
      # incomplete entry fails config parsing at startup.
      taxis.libraries = {
        music = {
          path = "/media/music";
          media_type = "music";
          watcher_mode = "auto";
          poll_interval_seconds = 300;
          scan_interval_hours = 24;
        };
        audiobooks = {
          path = "/media/audiobooks";
          media_type = "book";
          watcher_mode = "poll";
          poll_interval_seconds = 300;
          scan_interval_hours = 24;
        };
        books = {
          path = "/media/books";
          media_type = "book";
          watcher_mode = "poll";
          poll_interval_seconds = 300;
          scan_interval_hours = 24;
        };
      };

      # A non-secret example setting (epignosis.cache_ttl_secs is a plain
      # u64, unlike the provider API keys below it).
      epignosis.cache_ttl_secs = 86400;
    };
  };
}
```

Any `settings.*` field that holds real secret material (API keys such as
`epignosis.tmdb_key`, `syndesmos.lastfm.api_key`/`shared_secret`,
`syndesmos.plex.token`, `exousia.jwt_secret`) should be supplied via
`secretsFile` instead of `settings` — `settings` is rendered to
`${configFile}` in the Nix store, which is world-readable by default.
`secretsFile` values are merged on top of `settings` at load time (see
[architecture/configuration.md](architecture/configuration.md)), so any
field can be overridden that way regardless of which table it lives under.

## Module options

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

## Secret management

Pass `secretsFile` a path managed by [agenix](https://github.com/ryantm/agenix) or [sops-nix](https://github.com/Mic92/sops-nix). The file is delivered to the service via systemd `LoadCredential`, so it is never world-readable and the path in the environment variable (`HARMONIA_SECRETS_PATH`) points to the credential directory, not the original path. Horismos reads this variable directly (`crates/horismos/src/secrets.rs`) as an override for the sibling-of-config-file default — this is required here, since `harmonia.toml` lives in the read-only Nix store and has no writable sibling directory.

## Systemd hardening

The service runs with a hardened systemd profile:

- `NoNewPrivileges`, `PrivateTmp`, `PrivateDevices`
- `ProtectSystem = strict` with explicit `ReadWritePaths` for `dataDir`, `ergasia.download_dir`, `komide.podcast_dir`, and every configured library path
- `MemoryDenyWriteExecute`, `RestrictNamespaces`, `RestrictRealtime`
- `SystemCallFilter = @system-service ~@privileged`

## Overlay

To use the package in your own NixOS config without the module:

```nix
nixpkgs.overlays = [ inputs.harmonia.overlays.default ];
environment.systemPackages = [ pkgs.harmonia ];
```

The native flake package wraps the `harmonia` binary with ebook conversion tools
on `PATH`: `ebook-convert` from Calibre, `kepubify`, and `pandoc`. See
[`media/ebook-conversion.md`](media/ebook-conversion.md) for the runtime
contract.

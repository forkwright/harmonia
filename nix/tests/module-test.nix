{ pkgs }:

pkgs.nixosTest {
  name = "harmonia-basic";

  nodes.server = { ... }: {
    imports = [ ../module.nix ];

    # Supply the package from the local build; tests run with the overlay applied.
    nixpkgs.overlays = [
      (final: prev: {
        harmonia = pkgs.harmonia or (throw "harmonia package not in overlay — run tests via flake checks");
      })
    ];

    services.harmonia = {
      enable = true;
      settings.paroche.port = 8096;
      # WHY: exousia.jwt_secret is a hard boot-validation requirement
      # (crates/horismos/src/validation.rs: non-placeholder, >= 32 bytes) —
      # without it the service exits immediately. Delivered via
      # LoadCredential + HARMONIA_SECRETS_PATH (honored by horismos as of
      # forkwright/harmonia#610), never as plaintext in `settings` (which
      # lands in a world-readable Nix store path).
      secretsFile = pkgs.writeText "harmonia-test-secrets.toml" ''
        [exousia]
        jwt_secret = "nixos-test-only-jwt-secret-at-least-32-bytes-long"
      '';
    };
  };

  testScript = ''
    server.wait_for_unit("harmonia.service")
    server.wait_for_open_port(8096)
    server.succeed("curl -sf http://localhost:8096/api/system/health")
  '';
}

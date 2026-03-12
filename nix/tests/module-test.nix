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
    };
  };

  testScript = ''
    server.wait_for_unit("harmonia.service")
    server.wait_for_open_port(8096)
    server.succeed("curl -sf http://localhost:8096/api/system/health")
  '';
}

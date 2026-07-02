{
  description = "OMP Session Visualizer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        rustPkg = pkgs.rustPlatform.buildRustPackage {
          pname = "omp-visualizer";
          version = "0.1.0";
          src = ./backend;

          cargoLock = {
            lockFile = ./backend/Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [ pkg-config ];

          meta = with pkgs.lib; {
            description = "Visualization tool for OMP coding-agent sessions";
            license = licenses.mit;
          };
        };

        backendImage = pkgs.dockerTools.buildLayeredImage {
          name = "omp-visualizer-backend";
          tag = "latest";
          contents = [ rustPkg pkgs.cacert ];
          config = {
            Cmd = [ "${rustPkg}/bin/omp-visualizer" ];
            ExposedPorts = { "3000/tcp" = {}; };
            WorkingDir = "/";
          };
        };

        frontendImage = pkgs.dockerTools.buildLayeredImage {
          name = "omp-visualizer-frontend";
          tag = "latest";
          contents = [ pkgs.nginx pkgs.cacert ];
          config = {
            ExposedPorts = { "80/tcp" = {}; };
          };
          extraCommands = ''
            mkdir -p var/www
            cp -r ${./frontend/static}/* var/www/
            cat > etc/nginx/nginx.conf <<'EOF'
            events { worker_connections 1024; }
            http {
              include ${pkgs.nginx}/conf/mime.types;
              server {
                listen 80;
                location /api/ {
                  proxy_pass http://backend:3000;
                }
                location / {
                  proxy_pass http://backend:3000;
                }
              }
            }
            EOF
          '';
        };
      in
      {
        packages = {
          backend = backendImage;
          frontend = frontendImage;
          default = rustPkg;
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            rustc cargo rustfmt clippy
            pkg-config
            openssl
          ];
        };
      }
    );
}

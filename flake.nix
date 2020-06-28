{
  description = "nix written in rust";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "i686-linux" "x86_64-darwin" "aarch64-linux" ];

      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system);

      # Memoize nixpkgs for different platforms for efficiency.
      nixpkgsFor = forAllSystems (system:
        import nixpkgs {
          inherit system;
          overlays = [ self.overlay ];
        });

      commondDeps = pkgs: {
        buildDeps = with pkgs;
          [

            # TODO: tests
          ];

        propBuildDeps = with pkgs; [ sqlite ];
      };

      version = "0.1.0"; # TODO: version managemant

    in {
      overlay = final: prev: {
        rusty-nix = with final;
          with commondDeps pkgs;
          (rustPlatform.buildRustPackage {
            pname = "rusty-nix";
            inherit version;

            src = self;

            outputs = [ "out" "doc" ]; # TODO: dev/doc?

            buildInputs = buildDeps;

            propagatedBuildInputs = propBuildDeps;

            cargoSha256 =
              "af3738dabeba35e07b71f0b6c96c551ef46618971763360fbad804def32528ea";

            postInstall = ''
              cargo doc --workspace --release --all-features --frozen --offline --target-dir $doc

              mkdir -p $doc/nix-support/
              echo "doc manual $doc/" >> $doc/nix-support/hydra-build-products
            '';

          });
      };

      hydraJobs = {
        build =
          nixpkgs.lib.genAttrs systems (system: nixpkgsFor.${system}.rusty-nix);
      };

      packages =
        forAllSystems (system: { inherit (nixpkgsFor.${system}) rusty-nix; });

      defaultPackage =
        forAllSystems (system: self.packages.${system}.rusty-nix);
    };
}

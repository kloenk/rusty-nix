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
            sqlite
            # TODO: tests
          ];

        propBuildDeps = with pkgs; [ ];
      };

      version = "0.1.0"; # TODO: version managemant

    in {
      overlay = final: prev: {
        rusty-nix = with final;
          let
            lockFile = builtins.fromTOML (builtins.readFile
              ./Cargo.lock); # (builtins.readFile self + "./Cargo.lock");

            files = map (pkg:
              fetchurl {
                name = pkg.name;
                url =
                  "https://crates.io/api/v1/crates/${pkg.name}/${pkg.version}/download";
                sha256 = pkg.checksum;
              }) (builtins.filter (pkg:
                pkg.source or ""
                == "registry+https://github.com/rust-lang/crates.io-index")
                lockFile.package);
            vendorCrates = runCommand "cargo-vendor-dir" { } ''
              mkdir -p $out/vendor

              cat > $out/vendor/config << EOF
              [source.crates-io]
              replace-with = "vendored-sources"

              [source.vendored-sources]
              directory = "vendor"
              EOF
              ${toString (builtins.map (file: ''
                mkdir $out/vendor/tmp
                tar xvf ${file} -C $out/vendor/tmp
                dir=$(echo $out/vendor/tmp/*)

                printf '{"files":{},"package":"${file.outputHash}"}' > "$dir/.cargo-checksum.json"

                if [[ $dir =~ /winapi ]]; then
                  find $dir -name "*.a" -print0 | xargs -0 rm -f --
                fi

                mv "$dir" $out/vendor/

                rm -rf $out/vendor/tmp
              '') files)}
            '';
          in stdenv.mkDerivation {
            name = "rusty-nix";

            nativeBuildInputs = [ cargo rustc ];

            buildInputs = [ sqlite ];

            #propagatedBuildInputs = [

            #];

            buildPhase = ''
              ln -sfn ${vendorCrates}/vendor/ vendor
              export CARGO_HOME=$(pwd)/vendor
              cargo build --release --offline
            '';

            installPhase = ''
              install -D -m755 ./target/release/backend $out/bin/backend
            '';

            doCheck = true;
            checkPhase = ''
              cargo test --release
            '';
          };
        /* rusty-nix = with final;
                     with commondDeps pkgs;
                     (rustPlatform.buildRustPackage {
                       pname = "rusty-nix";
                       inherit version;

                       src = self;

                       #outputs = [ "out" "doc" ]; # TODO: dev/doc?

                       buildInputs = buildDeps;

                       propagatedBuildInputs = propBuildDeps;

                       cargoSha256 =
                         "sha256-nPT3yNS5Lfn6GwI8Zjatik8qxXFoD80kZGGHYrnx99Q=";

           #            postInstall = ''
           #              cargo doc --workspace --release --all-features --frozen --offline --target-dir $doc
           #
           #              mkdir -p $doc/nix-support/
           #              echo "doc manual $doc/" >> $doc/nix-support/hydra-build-products
           #            '';

                     });
        */
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

{
    description = "ytermusic — a TUI YouTube Music player (with browser-cookie API fix)";

    inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    outputs =
        { self, nixpkgs }:
        let
            systems = [
                "x86_64-linux"
                "aarch64-linux"
            ];
            forAll = nixpkgs.lib.genAttrs systems;
            pkgsFor = system: nixpkgs.legacyPackages.${system};

            mkPackage =
                pkgs:
                pkgs.rustPlatform.buildRustPackage {
                    pname = "ytermusic";
                    version = "master";
                    src = ./.;
                    # Resolve deps from the committed Cargo.lock (avoids a stale cargoHash
                    # when src/deps change); allowBuiltinFetchGit handles the rusty_ytdl git dep.
                    cargoLock = {
                        lockFileContents = builtins.readFile ./Cargo.lock;
                        allowBuiltinFetchGit = true;
                    };
                    cargoBuildType = "release";
                    # needs network for metadata; upstream also disables it
                    doCheck = false;
                    nativeBuildInputs = [
                        pkgs.gitMinimal
                        pkgs.pkg-config
                    ];
                    buildInputs = [
                        pkgs.openssl
                        pkgs.alsa-lib
                        pkgs.dbus
                    ];
                    meta = {
                        description = "TUI based Youtube Music Player that aims to be as fast and simple as possible";
                        homepage = "https://github.com/ccgauche/ytermusic";
                        license = pkgs.lib.licenses.asl20;
                        mainProgram = "ytermusic";
                        platforms = pkgs.lib.platforms.linux;
                    };
                };
        in
        {
            packages = forAll (system: {
                default = mkPackage (pkgsFor system);
            });

            devShells = forAll (
                system:
                let
                    pkgs = pkgsFor system;
                in
                {
                    default = pkgs.mkShell {
                        packages = [
                            pkgs.rustc
                            pkgs.cargo
                            pkgs.rustfmt
                            pkgs.clippy
                            pkgs.rust-analyzer
                            pkgs.pkg-config
                            pkgs.openssl
                            pkgs.alsa-lib
                            pkgs.dbus
                            pkgs.yt-dlp # the downloader backend used at build/runtime
                        ];
                    };
                }
            );

            apps = forAll (system: {
                default = {
                    type = "app";
                    program = "${self.packages.${system}.default}/bin/ytermusic";
                };
            });
        };
}

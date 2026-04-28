{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";

    impeller.url = "github:leanprover/impeller";
    impeller.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    inputs:
    let
      lib = inputs.nixpkgs.lib;
      forAllSystems = lib.genAttrs lib.systems.flakeExposed;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = inputs.nixpkgs.legacyPackages.${system};
          craneLib = inputs.crane.mkLib pkgs;
          impeller = inputs.impeller.packages.${system}.default;
        in
        {
          default = craneLib.buildPackage {
            src = ./.;
            buildInputs = [ impeller ];
            nativeBuildInputs = [ pkgs.makeWrapper ];
            postInstall = ''
              wrapProgram $out/bin/pump \
                --prefix PATH : ${lib.makeBinPath [ impeller ]}
            '';
          };
        }
      );
    };
}

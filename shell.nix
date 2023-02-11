let pkgs = import <nixpkgs> {};

in pkgs.mkShell rec {
  name = "tiberius";
  
  nativeBuildInputs = with pkgs; [
    nodejs-14_x
    cargo-cross
    #rustup
    sqlx-cli
    (yarn.override { nodejs = nodejs-14_x; })
    (nodePackages.npm.override { nodejs = nodejs-14_x; })
  ];
}

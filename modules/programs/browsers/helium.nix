{ inputs, ... }:
{
  flake.modules.homeManager.browsers =
    { ... }:
    {
      imports = [ inputs.helium.homeModules.default ];

      programs.helium = {
        enable = true;

        # CLI flags passed to the browser
        commandLineArgs = [
          "--ozone-platform-hint=auto"
          "--enable-features=WaylandWindowDecorations"
          "--enable-wayland-ime=true"
        ];

        extensions = [
          { id = "ghmbeldphafepmbegfdlkpapadhbakde"; } # Proton Pass
          { id = "cjpalhdlnbpafiamejdnhcphjbkeiagm"; } # uBlock Origin
          { id = "lnjaiaapbakfhlbjenjkhffcdpoompki"; } # Catppuccin for Web File Explorer Icons
          { id = "dnhpnfgdlenaccegplpojghhmaamnnfp"; } # Augmented Steam
          { id = "mnjggcdmjocbbbhaepdhchncahnbgone"; } # SponsorBlock
          { id = "ngonfifpkpeefnhelnfdkficaiihklid"; } # ProtonDB for Steam
          { id = "kdbmhfkmnlmbkgbabkdealhhbfhlmmon"; } # SteamDB
          { id = "oeakphpfoaeggagmgphfejmfjbhjfhhh"; } # YT Tweaks
        ];

        # nativeMessagingHosts = [ pkgs.browserpass ];
      };
    };
}

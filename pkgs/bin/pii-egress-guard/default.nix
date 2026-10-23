{
  lib,
  stdenvNoCC,
  python3,
}:
stdenvNoCC.mkDerivation {
  pname = "pii-egress-guard";
  version = "0.1.0";
  src = ./.;

  dontBuild = true;
  doCheck = true;
  nativeCheckInputs = [ python3 ];

  checkPhase = ''
    runHook preCheck
    python3 -I -m unittest discover -s tests -v
    runHook postCheck
  '';

  installPhase = ''
    runHook preInstall
    install -Dm755 pii_egress_guard.py $out/bin/pii-egress-guard
    substituteInPlace $out/bin/pii-egress-guard \
      --replace-fail "#!/usr/bin/env python3" "#!${python3.interpreter} -I"
    runHook postInstall
  '';

  meta = {
    description = "Claude Code PreToolUse hook that asks before tool calls send personal identifiers off the machine";
    homepage = "https://github.com/guppybot161/ai-tools/tree/main/hooks/pii-egress-guard";
    mainProgram = "pii-egress-guard";
    platforms = lib.platforms.unix;
  };
}

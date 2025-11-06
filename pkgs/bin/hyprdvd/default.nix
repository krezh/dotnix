{
  lib,
  python3,
  fetchFromGitHub,
}:

python3.pkgs.buildPythonApplication rec {
  pname = "hyprdvd";
  version = "0.5.0";
  pyproject = true;

  src = fetchFromGitHub {
    owner = "nevimmu";
    repo = "hyprdvd";
    rev = version;
    hash = "sha256-my33xuCtZLLOlFEiKUmMz3SLSVxtkrCSdvMSKAhaodM=";
  };

  build-system = [
    python3.pkgs.setuptools
  ];

  dependencies = with python3.pkgs; [
    argcomplete
  ];

  pythonImportsCheck = [ ];

  meta = {
    description = "Bounce your terminal like a DVD screen";
    homepage = "https://github.com/nevimmu/hyprdvd";
    changelog = "https://github.com/nevimmu/hyprdvd/blob/${src.rev}/CHANGELOG.md";
    license = lib.licenses.mit;
    mainProgram = "hyprdvd";
  };
}

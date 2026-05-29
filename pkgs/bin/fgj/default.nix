{
  lib,
  buildGoModule,
  fetchFromGitea,
  nix-update-script,
}:

buildGoModule (finalAttrs: {
  pname = "fgj";
  version = "0.4.0";
  __structuredAttrs = true;

  src = fetchFromGitea {
    domain = "codeberg.org";
    owner = "romaintb";
    repo = "fgj";
    tag = "v${finalAttrs.version}";
    hash = "sha256-7/ITo+8QCj/hy4xlOw+kfjnJbHTWjGh+VYOZxvqghAQ=";
  };

  vendorHash = "sha256-ZBdSSif9YFpFyBQNpZ/XttVw/dgDS54L+0ZA+9ObSSg=";

  ldflags = [ "-s" ];

  passthru.updateScript = nix-update-script { };

  meta = {
    description = "Forgejo CLI tool - work seamlessly with Forgejo and Codeberg from the command line. Like `gh` for GitHub, but for Forgejo instances";
    homepage = "https://codeberg.org/romaintb/fgj";
    changelog = "https://codeberg.org/romaintb/fgj/releases/tag/${finalAttrs.src.tag}";
    license = lib.licenses.mit;
    maintainers = with lib.maintainers; [ ];
    mainProgram = "fgj";
  };
})

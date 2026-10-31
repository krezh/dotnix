# Personal preferences

- I always run the latest versions of all software (a personal habit, not project-specific).
  Your training data is outdated: never write a version number, flag, option, or config key from memory.
  Verify it first against a current source (upstream releases/changelog, official docs, nixpkgs) and use what that source says.
  Write config syntax, APIs, flags, and features for the newest release.
  Don't suggest deprecated, archived, or unmaintained projects, and don't hedge about older-version compatibility unless I ask.

# Tools

- Before assuming a capability isn't available, call the `mcp-tools` MCP server's `find_tool` to search its tool catalog, then `call_tool` to run whatever it finds.
  Do this proactively, without being asked, whenever a task needs something outside your built-in tools (infra/homelab integrations, etc.).
- Check for a jj repo with `jj root` (not file tools or globs; they hide `.jj`).
  Inside a jj repo, NEVER run `git` commands (`git log`, `git diff`, `git show`, etc.); use the `jj` equivalents.

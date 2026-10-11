# Personal preferences
- I always run the latest versions of all software (this is a personal habit, not project-specific).
  When choosing config syntax, APIs, flags, or features, assume the newest release.
  Don't suggest legacy/older alternatives or hedge about version compatibility unless I ask.

# Tools
- Before assuming a capability isn't available, call the `mcp-tools` MCP server's `find_tool` to search its tool catalog, then `call_tool` to run whatever it finds.
  Do this proactively, without being asked, whenever a task needs something outside your built-in tools (infra/homelab integrations, etc.).
- Always use jj if a .jj directory exists in the project root

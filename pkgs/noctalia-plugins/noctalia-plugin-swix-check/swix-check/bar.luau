--!nonstrict
-- Bar badge with the number of incoming upstream changes; click opens swix.

-- Monochrome NixOS snowflake; `#000000` is replaced with the theme color it is drawn in.
local ICON_TEMPLATE = noctalia.readFile("nixos.svg")
local ICON_SIZE = 18
-- Tinted icon path per theme hex color, written once into the plugin data dir.
local iconPaths = {}

local report = noctalia.state.get("report")
local lastError = noctalia.state.get("error")
local polling = noctalia.state.get("polling") == true

local function buildTooltip()
  local lines = {}
  if report then
    table.insert(lines, string.format("%d incoming, %d local", report.incoming, report["local"]))
    table.insert(lines, string.format("%s %s", report.vcs, report.upstream))
    table.insert(lines, "Checked " .. noctalia.formatTime("{:%H:%M}", report.checkedAt))
  elseif not lastError then
    table.insert(lines, "Checking…")
  end
  if lastError then
    table.insert(lines, "Error: " .. tostring(lastError))
  end
  table.insert(lines, "")
  table.insert(lines, "Click to open swix · Right-click to check now")
  return table.concat(lines, "\n")
end

local function iconPath(role)
  local hex = noctalia.getColor(role)
  if not hex or not ICON_TEMPLATE then
    return "nixos.svg"
  end
  local path = iconPaths[hex]
  if path then
    return path
  end
  local dir = noctalia.pluginDataDir()
  if not dir then
    return "nixos.svg"
  end
  path = string.format("%s/nixos-%s.svg", dir, hex:sub(2):lower())
  if not noctalia.fileExists(path) then
    noctalia.writeFile(path, (ICON_TEMPLATE:gsub("#000000", hex)))
  end
  iconPaths[hex] = path
  return path
end

local function render()
  local incoming = report and report.incoming or 0
  local color = "on_surface_variant"
  if lastError then
    color = "error"
  elseif incoming > 0 then
    color = "primary"
  end

  barWidget.setTooltip(buildTooltip())

  local children = {
    ui.image({
      path = iconPath(color),
      width = ICON_SIZE,
      height = ICON_SIZE,
      fit = "contain",
    }),
  }

  if incoming > 0 or polling then
    table.insert(children, ui.label({
      text = incoming > 0 and tostring(incoming) or "…",
      fontSize = 12,
      fontWeight = "bold",
      color = color,
    }))
  end

  local isVertical = (barWidget.isVertical and barWidget.isVertical()) or false
  local container = isVertical and ui.column or ui.row
  barWidget.render(container({
    gap = 5,
    align = "center",
    paddingH = 3,
  }, children))
end

noctalia.state.watch("report", function(val)
  report = val
  render()
end)

noctalia.state.watch("error", function(val)
  lastError = val
  render()
end)

noctalia.state.watch("polling", function(val)
  polling = val == true
  render()
end)

function onClick()
  noctalia.runAsync({ "swix" })
end

-- Re-render periodically so the icon follows theme palette changes.
function update()
  render()
end

noctalia.setUpdateInterval(10000)

render()

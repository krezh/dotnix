--!nonstrict
-- Bar indicator with one glyph and battery level per headset; hidden while no headset with a
-- battery is connected. A headset that is switched off shows its glyph without a level.

local STATUS_LABELS = {
  BATTERY_AVAILABLE = "On battery",
  BATTERY_CHARGING = "Charging",
  BATTERY_UNAVAILABLE = "Battery unavailable (headset off?)",
}

local headsets = noctalia.state.get("headsets")
local lastError = noctalia.state.get("error")
local absent = noctalia.state.get("absent") == true
local lowPercent = tonumber(noctalia.getConfig("low_percent")) or 20
local glyph = noctalia.getConfig("glyph") or "headphones"

local function hasLevel(headset)
  return headset.status ~= "BATTERY_UNAVAILABLE" and headset.level >= 0
end

local function buildTooltip()
  local lines = {}
  for _, headset in ipairs(headsets and headsets.list or {}) do
    if #lines > 0 then
      table.insert(lines, "")
    end
    table.insert(lines, headset.name)
    local status = STATUS_LABELS[headset.status] or headset.status
    if hasLevel(headset) then
      status = string.format("%d%% · %s", headset.level, status)
    end
    table.insert(lines, status)
    if headset.voltageMv then
      table.insert(lines, string.format("%.2f V", headset.voltageMv / 1000))
    end
    if headset.timeToFullMin then
      table.insert(lines, string.format("Full in %d min", headset.timeToFullMin))
    end
    if headset.timeToEmptyMin then
      table.insert(lines, string.format("Empty in %d min", headset.timeToEmptyMin))
    end
    for _, err in ipairs(headset.errors) do
      table.insert(lines, "Error: " .. err)
    end
  end
  if headsets then
    table.insert(lines, "Updated " .. noctalia.formatTime("{:%H:%M}", headsets.checkedAt))
  elseif not lastError then
    table.insert(lines, "Reading…")
  end
  if lastError then
    table.insert(lines, "Error: " .. tostring(lastError))
  end
  table.insert(lines, "")
  table.insert(lines, "Right-click to refresh")
  return table.concat(lines, "\n")
end

local function headsetColor(headset)
  if lastError or (hasLevel(headset) and headset.level <= lowPercent) then
    return "error"
  elseif headset.status == "BATTERY_CHARGING" then
    return "primary"
  end
  return "on_surface_variant"
end

local function render()
  barWidget.setVisible(not absent)
  if absent then
    return
  end

  barWidget.setTooltip(buildTooltip())

  local children = {}
  local list = headsets and headsets.list or {}
  if #list == 0 then
    -- First query still running, or it failed before any headset was seen.
    local color = lastError and "error" or "on_surface_variant"
    table.insert(children, ui.glyph({ name = glyph, size = 18, color = color }))
    table.insert(children, ui.label({ text = "…", fontSize = 12, fontWeight = "bold", color = color }))
  end
  for _, headset in ipairs(list) do
    local color = headsetColor(headset)
    table.insert(children, ui.glyph({ name = glyph, size = 18, color = color }))
    if hasLevel(headset) then
      table.insert(children, ui.label({
        text = string.format("%d%%", headset.level),
        fontSize = 12,
        fontWeight = "bold",
        color = color,
      }))
    end
  end

  local container = barWidget.isVertical() and ui.column or ui.row
  barWidget.render(container({
    gap = 4,
    align = "center",
    paddingH = 3,
  }, children))
end

noctalia.state.watch("headsets", function(val)
  headsets = val
  render()
end)

noctalia.state.watch("error", function(val)
  lastError = val
  render()
end)

noctalia.state.watch("absent", function(val)
  absent = val == true
  render()
end)

function onConfigChanged()
  lowPercent = tonumber(noctalia.getConfig("low_percent")) or 20
  glyph = noctalia.getConfig("glyph") or "headphones"
  render()
end

render()

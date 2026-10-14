--!nonstrict
-- Bar indicator for the ZOWIE mouse battery; hidden while no receiver or wired mouse is present.

local POWER_LABELS = {
  usb = "Wired, on USB power",
  external = "Wired, on external power",
  battery = "Wired, on battery",
}

local reading = noctalia.state.get("reading")
local lastError = noctalia.state.get("error")
local absent = noctalia.state.get("absent") == true
local lowPercent = tonumber(noctalia.getConfig("low_percent")) or 20

local function buildTooltip()
  local lines = { "ZOWIE mouse" }
  if reading then
    table.insert(lines, string.format("~%d%% · %.2f V", reading.percent, reading.voltageMv / 1000))
    table.insert(lines, POWER_LABELS[reading.power] or "Wireless")
    table.insert(lines, "Updated " .. noctalia.formatTime("{:%H:%M}", reading.checkedAt))
  elseif not lastError then
    table.insert(lines, "Reading…")
  end
  if lastError then
    table.insert(lines, "Error: " .. tostring(lastError))
  end
  table.insert(lines, "")
  table.insert(lines, "Percentage is estimated from voltage · Right-click to refresh")
  return table.concat(lines, "\n")
end

local function render()
  barWidget.setVisible(not absent)
  if absent then
    return
  end

  local wired = reading ~= nil and reading.connection == "wired"
  local color = "on_surface_variant"
  if lastError or (reading and reading.percent <= lowPercent) then
    color = "error"
  elseif wired then
    color = "primary"
  end

  barWidget.setTooltip(buildTooltip())

  local container = barWidget.isVertical() and ui.column or ui.row
  barWidget.render(container({
    gap = 4,
    align = "center",
    paddingH = 3,
  }, {
    ui.glyph({
      name = wired and "battery-charging" or "mouse",
      size = 18,
      color = color,
    }),
    ui.label({
      text = reading and string.format("%d%%", reading.percent) or "…",
      fontSize = 12,
      fontWeight = "bold",
      color = color,
    }),
  }))
end

noctalia.state.watch("reading", function(val)
  reading = val
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
  render()
end

render()

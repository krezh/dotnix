--!nonstrict
-- OMP Usage clickable bar icon
-- Minimal, native bar widget with legible typography and high-contrast alert states

local highUsageThreshold = tonumber(noctalia.getConfig("high_usage_threshold")) or 75
local glyph = noctalia.getConfig("glyph") or "chart-donut-3"
local showBadge = noctalia.getConfig("show_badge") ~= false

local report = noctalia.state.get("report")
local polling = noctalia.state.get("polling") == true

local function formatCountdown(resetsAtMs)
  if not resetsAtMs or resetsAtMs <= 0 then
    return "--"
  end
  local nowMs = (noctalia.nowMs and noctalia.nowMs()) or (os.time() * 1000)
  local diffMs = resetsAtMs - nowMs
  if diffMs <= 0 then
    return "now"
  end

  local sec = math.floor(diffMs / 1000)
  if sec < 60 then
    return sec .. "s"
  end

  local min = math.floor(sec / 60)
  if min < 60 then
    return min .. "m"
  end

  local hours = math.floor(min / 60)
  local remMin = min % 60
  if hours < 24 then
    return hours .. "h " .. remMin .. "m"
  end

  local days = math.floor(hours / 24)
  local remHours = hours % 24
  return days .. "d " .. remHours .. "h"
end

local function getHighUsageLimits()
  local list = {}
  if not report or not report.providers then
    return list
  end

  for _, p in ipairs(report.providers) do
    for _, lim in ipairs(p.limits or {}) do
      if lim.status == "exhausted" or lim.used >= highUsageThreshold then
        table.insert(list, lim)
      end
    end
  end

  table.sort(list, function(a, b)
    local aEx = (a.status == "exhausted" or a.used >= 95)
    local bEx = (b.status == "exhausted" or b.used >= 95)
    if aEx and not bEx then
      return true
    elseif not aEx and bEx then
      return false
    elseif aEx and bEx then
      if a.resetsAt and b.resetsAt then
        return a.resetsAt < b.resetsAt
      end
    end
    return a.used > b.used
  end)

  return list
end

local function buildTooltip(highUsageList)
  local lines = {}
  table.insert(lines, "OMP Provider Quotas")
  table.insert(lines, string.rep("─", 24))

  if highUsageList and #highUsageList > 0 then
    for _, lim in ipairs(highUsageList) do
      local cd = formatCountdown(lim.resetsAt)
      local statusTag = (lim.status == "exhausted" or lim.used >= 95) and " [EXHAUSTED]" or ""
      table.insert(lines, string.format("• %s (%s): %.0f%% (resets in %s)%s", lim.displayName, lim.label, lim.used, cd, statusTag))
    end
  else
    table.insert(lines, "All quotas within safe limits.")
  end

  table.insert(lines, "")
  table.insert(lines, "Click to open · Right-click to refresh")
  return table.concat(lines, "\n")
end

local function render()
  local highUsageList = getHighUsageLimits()
  local isVertical = (barWidget.isVertical and barWidget.isVertical()) or false
  local container = isVertical and ui.column or ui.row

  barWidget.setTooltip(buildTooltip(highUsageList))

  local children = {}

  if #highUsageList > 0 then
    local worst = highUsageList[1]
    local isExhausted = (worst.status == "exhausted" or worst.used >= 95)
    local alertColor = isExhausted and "error" or "warning"
    local activeGlyph = worst.glyph or glyph

    table.insert(children, ui.glyph({
      name = activeGlyph,
      size = 20,
      color = alertColor,
    }))

    if showBadge then
      local badgeText = formatCountdown(worst.resetsAt)
      if badgeText == "--" then
        badgeText = string.format("%.0f%%", worst.used)
      end

      table.insert(children, ui.row({
        fill = "surface_variant/0.50",
        radius = 6,
        paddingH = 6,
        paddingV = 2,
        align = "center",
      }, {
        ui.label({
          text = badgeText,
          fontSize = 12,
          fontWeight = "bold",
          color = alertColor,
        }),
      }))
    end
  else
    local iconColor = polling and "primary" or "on_surface_variant"
    table.insert(children, ui.glyph({
      name = glyph,
      size = 20,
      color = iconColor,
    }))

    if polling then
      table.insert(children, ui.label({
        text = "…",
        fontSize = 12,
        fontWeight = "bold",
        color = "primary",
      }))
    end
  end

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

noctalia.state.watch("polling", function(val)
  polling = val == true
  render()
end)

function update()
  noctalia.setUpdateInterval(5000)
  render()
end

function onClick()
  noctalia.togglePanel("krezh/omp-usage:panel")
end

function onRightClick()
  noctalia.state.set("command", { action = "refresh" })
end

function onConfigChanged()
  highUsageThreshold = tonumber(noctalia.getConfig("high_usage_threshold")) or 75
  glyph = noctalia.getConfig("glyph") or "chart-donut-3"
  showBadge = noctalia.getConfig("show_badge") ~= false
  render()
end

render()

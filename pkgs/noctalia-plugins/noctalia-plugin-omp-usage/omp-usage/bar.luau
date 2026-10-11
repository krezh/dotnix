--!nonstrict
-- OMP Usage clickable bar icon
-- "worst" mode shows the most urgent alert, healthy icon otherwise; "providers" mode shows
-- one glyph per provider with its headline usage.

local shared = require("./shared.luau")

local glyph = noctalia.getConfig("glyph") or "chart-donut-3"
local showBadge = noctalia.getConfig("show_badge") ~= false
local display = noctalia.getConfig("display") or "worst"

local report = noctalia.state.get("report")
local polling = noctalia.state.get("polling") == true
local lastError = noctalia.state.get("last_error")

local function buildTooltip(alerts)
  local lines = { "OMP Provider Quotas", string.rep("─", 24) }

  if lastError and lastError ~= "" then
    table.insert(lines, "⚠ Last refresh failed; data may be stale:")
    table.insert(lines, "  " .. lastError)
    table.insert(lines, "")
  end

  if #alerts > 0 then
    for _, lim in ipairs(alerts) do
      local statusTag = lim.severity == "exhausted" and " [EXHAUSTED]" or ""
      table.insert(lines, string.format("• %s (%s): %.0f%% (resets in %s)%s",
        lim.displayName, lim.label, lim.used, shared.formatCountdown(lim.resetsAt), statusTag))
      if lim.exhaustsAt then
        table.insert(lines, "    on pace to run out in " .. shared.formatCountdown(lim.exhaustsAt))
      end
    end
  elseif report then
    table.insert(lines, "All quotas within safe limits.")
  else
    table.insert(lines, "No usage data yet.")
  end

  local issues = (report and report.issues) or {}
  if #issues > 0 then
    table.insert(lines, "")
    table.insert(lines, string.format("⚠ %d account(s) without usage data", #issues))
  end

  table.insert(lines, "")
  table.insert(lines, "Click to open · Right-click to refresh")
  return table.concat(lines, "\n")
end

local function badge(text, color)
  return ui.label({
    text = text,
    fontSize = 12,
    fontWeight = "bold",
    color = color,
  })
end

local function idleIcon(children)
  local iconColor = "on_surface_variant"
  if lastError and lastError ~= "" then
    iconColor = "error"
  elseif polling then
    iconColor = "primary"
  end
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

local function worstChildren(alerts)
  local children = {}
  local worst = alerts[1]
  if not worst then
    idleIcon(children)
    return children
  end

  local alertColor = shared.severityColor(worst.severity)
  table.insert(children, ui.glyph({
    name = worst.glyph or glyph,
    size = 20,
    color = alertColor,
  }))

  if showBadge then
    local badgeText = shared.formatCountdown(worst.resetsAt)
    if badgeText == "--" then
      badgeText = string.format("%.0f%%", worst.used)
    end
    table.insert(children, badge(badgeText, alertColor))
  end
  return children
end

-- Exhausted providers show time until reset, others their headline usage.
local function providerChildren()
  local children = {}
  local providers = (report and report.providers) or {}
  if #providers == 0 then
    idleIcon(children)
    return children
  end

  for _, prov in ipairs(providers) do
    local color = prov.severity == "ok" and "on_surface_variant" or shared.severityColor(prov.severity)
    table.insert(children, ui.glyph({ name = prov.glyph, size = 18, color = color }))
    local headline = prov.headline
    if headline then
      local text = headline.severity == "exhausted"
        and shared.formatCountdown(headline.resetsAt)
        or string.format("%.0f%%", headline.used)
      table.insert(children, ui.label({
        text = text,
        fontSize = 12,
        fontWeight = "bold",
        color = color,
      }))
    end
  end
  return children
end

local function render()
  local alerts = (report and report.alerts) or {}
  local container = barWidget.isVertical() and ui.column or ui.row

  barWidget.setTooltip(buildTooltip(alerts))

  local children = display == "providers" and providerChildren() or worstChildren(alerts)
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

noctalia.state.watch("last_error", function(val)
  lastError = val
  render()
end)

-- Ticks keep the countdown badge current.
function update()
  render()
end

function onClick()
  noctalia.togglePanel("krezh/omp-usage:panel")
end

function onRightClick()
  noctalia.state.set("command", { action = "refresh" })
end

function onConfigChanged()
  glyph = noctalia.getConfig("glyph") or "chart-donut-3"
  showBadge = noctalia.getConfig("show_badge") ~= false
  display = noctalia.getConfig("display") or "worst"
  render()
end

noctalia.setUpdateInterval(5000)
render()

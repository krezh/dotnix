--!nonstrict
-- OMP Usage full floating/attached dashboard panel (720x800)
-- Clean typography, neutral background surfaces, zero colored-on-colored background fills

local report = noctalia.state.get("report")
local polling = noctalia.state.get("polling") == true
local lastError = noctalia.state.get("last_error")
local highUsageThreshold = tonumber(noctalia.getConfig("high_usage_threshold")) or 75

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

local function formatExactReset(resetsAtMs)
  if not resetsAtMs or resetsAtMs <= 0 then
    return ""
  end
  local nowMs = (noctalia.nowMs and noctalia.nowMs()) or (os.time() * 1000)
  local diffSec = math.floor((resetsAtMs - nowMs) / 1000)
  local resetSec = math.floor(resetsAtMs / 1000)

  if diffSec < 86400 then
    return "at " .. os.date("%H:%M", resetSec)
  else
    return os.date("%a %b %d at %H:%M", resetSec)
  end
end

local function formatUpdated(fetchedAt)
  if not fetchedAt or fetchedAt <= 0 then
    return ""
  end
  local now = os.time()
  local diffMin = math.floor((now - fetchedAt) / 60)
  if diffMin < 1 then
    return "Synced just now"
  end
  return string.format("Synced %dm ago", diffMin)
end

local function usageColor(used, status)
  if status == "exhausted" or used >= 95 then
    return "error"
  elseif used >= 75 then
    return "warning"
  else
    return "primary"
  end
end

local function providerUrgency(prov)
  local score = 0
  for _, lim in ipairs(prov.limits or {}) do
    if lim.status == "exhausted" or lim.used >= 95 then
      score = math.max(score, 10000 + (100 - (lim.used or 0)))
    elseif lim.used >= highUsageThreshold then
      score = math.max(score, 5000 + (lim.used or 0))
    else
      score = math.max(score, lim.used or 0)
    end
  end
  return score
end

-- Spacious 2-tier limit card with clean neutral backgrounds
local function buildLimitBlock(lim)
  local fraction = math.min(1.0, math.max(0.0, (lim.used or 0) / 100.0))
  local color = usageColor(lim.used, lim.status)
  local countdownStr = formatCountdown(lim.resetsAt)
  local exactTime = formatExactReset(lim.resetsAt)
  local isStrained = (lim.status == "exhausted" or lim.used >= highUsageThreshold)

  local percentStr = string.format("%.0f%%", lim.used)
  if lim.used > 0 and lim.used < 1 then
    percentStr = string.format("%.1f%%", lim.used)
  end

  local resetFull = "Resets in " .. countdownStr
  if exactTime ~= "" then
    resetFull = resetFull .. " (" .. exactTime .. ")"
  end

  return ui.column({
    padding = 12,
    radius = 10,
    fill = "surface_variant/0.20",
    gap = 8,
  }, {
    -- Top line: Label + Spacer + Percentage (plain bold text, no colored background)
    ui.row({ align = "center" }, {
      ui.label({
        text = lim.label,
        fontSize = 15,
        fontWeight = "bold",
        color = "on_surface",
      }),
      ui.spacer({ flexGrow = 1 }),
      ui.label({
        text = percentStr .. " used",
        fontSize = 14,
        fontWeight = "bold",
        color = color,
      }),
    }),

    -- Middle: Full-width Progress Bar
    ui.progress({
      progress = fraction,
      fill = color,
      track = "on_surface/0.14",
      height = 8,
      radius = 4,
    }),

    -- Bottom line: Reset Countdown with Clock + Remaining quota
    ui.row({ gap = 6, align = "center" }, {
      ui.glyph({
        name = "clock",
        size = 14,
        color = isStrained and color or "on_surface_variant",
      }),
      ui.label({
        text = resetFull,
        fontSize = 13,
        fontWeight = isStrained and "bold" or "medium",
        color = isStrained and color or "on_surface",
      }),
      ui.spacer({ flexGrow = 1 }),
      lim.status == "exhausted" and ui.label({
        text = "EXHAUSTED",
        fontSize = 12,
        fontWeight = "bold",
        color = "error",
      }) or ui.label({
        text = string.format("%.1f%% remaining", lim.remaining),
        fontSize = 12,
        color = "on_surface_variant",
      }),
    }),
  })
end

local function buildProviderCard(prov)
  local isExhausted = false
  for _, lim in ipairs(prov.limits or {}) do
    if lim.status == "exhausted" or lim.used >= 95 then
      isExhausted = true
      break
    end
  end

  local statusText = isExhausted and "EXHAUSTED" or "ACTIVE"
  local statusColor = isExhausted and "error" or "secondary"

  local accountSubtitle = prov.email
  if prov.planType and prov.planType ~= "" then
    if accountSubtitle ~= "" then
      accountSubtitle = accountSubtitle .. " · " .. prov.planType:upper()
    else
      accountSubtitle = prov.planType:upper()
    end
  end

  local limitItems = {}
  for _, lim in ipairs(prov.limits or {}) do
    table.insert(limitItems, buildLimitBlock(lim))
  end

  if #limitItems == 0 then
    table.insert(limitItems, ui.label({
      text = "No limits reported for this account",
      fontSize = 13,
      color = "on_surface_variant",
    }))
  end

  return ui.column({
    padding = 16,
    radius = 14,
    fill = "surface_variant/0.28",
    gap = 12,
  }, {
    -- Header Row: Icon + Name + Plan + Status
    ui.row({ align = "center", gap = 12 }, {
      ui.row({
        fill = "surface_variant/0.45",
        radius = 10,
        padding = 8,
        align = "center",
        justify = "center",
      }, {
        ui.glyph({
          name = prov.glyph or "brain",
          size = 22,
          color = isExhausted and "error" or "primary",
        }),
      }),
      ui.column({ gap = 2, flexGrow = 1 }, {
        ui.label({
          text = prov.displayName,
          fontSize = 16,
          fontWeight = "bold",
          color = "on_surface",
        }),
        accountSubtitle ~= "" and ui.label({
          text = accountSubtitle,
          fontSize = 12,
          color = "on_surface_variant",
        }) or ui.label({ text = "" }),
      }),
      ui.label({
        text = statusText,
        fontSize = 13,
        fontWeight = "bold",
        color = statusColor,
      }),
    }),

    -- Divider
    ui.box({
      height = 1,
      fill = "outline/0.12",
    }),

    -- Limit blocks
    ui.column({ gap = 8 }, limitItems),
  })
end

local function render()
  local contentElements = {}

  -- Top Header
  local syncText = polling and "Syncing..." or (report and formatUpdated(report.fetchedAt) or "")

  table.insert(contentElements, ui.row({ align = "center", gap = 12 }, {
    ui.row({
      fill = "surface_variant/0.45",
      radius = 10,
      padding = 8,
      align = "center",
      justify = "center",
    }, {
      ui.glyph({
        name = "chart-donut-3",
        size = 22,
        color = "primary",
      }),
    }),
    ui.column({ gap = 1, flexGrow = 1 }, {
      ui.label({
        text = "OMP Provider Quotas & Resets",
        fontSize = 18,
        fontWeight = "bold",
        color = "on_surface",
      }),
      ui.label({
        text = "Real-time allowances, usage percentage, and unlock times",
        fontSize = 13,
        color = "on_surface_variant",
      }),
    }),
    syncText ~= "" and ui.label({
      text = syncText,
      fontSize = 12,
      fontWeight = "medium",
      color = "on_surface_variant",
    }) or ui.label({ text = "" }),
    ui.button({
      glyph = "refresh",
      glyphSize = 16,
      variant = "ghost",
      controlSize = "md",
      tooltip = "Refresh quotas",
      onClick = function()
        noctalia.state.set("command", { action = "refresh" })
      end,
    }),
    ui.button({
      glyph = "close",
      glyphSize = 16,
      variant = "ghost",
      controlSize = "md",
      tooltip = "Close",
      onClick = function()
        if type(panel.close) == "function" then
          panel.close()
        end
      end,
    }),
  }))

  table.insert(contentElements, ui.box({ height = 1, fill = "outline/0.14" }))

  -- Error if any
  if lastError and lastError ~= "" then
    table.insert(contentElements, ui.row({
      padding = 10,
      radius = 10,
      fill = "error/0.16",
      align = "center",
      gap = 8,
    }, {
      ui.glyph({ name = "alert-circle", size = 16, color = "error" }),
      ui.label({ text = lastError, fontSize = 13, color = "error", maxLines = 2 }),
    }))
  end

  -- Sorted Provider Cards
  local providers = (report and report.providers) or {}
  local sortedProviders = {}
  for _, p in ipairs(providers) do
    table.insert(sortedProviders, p)
  end
  table.sort(sortedProviders, function(a, b)
    return providerUrgency(a) > providerUrgency(b)
  end)

  local cards = {}
  if #sortedProviders > 0 then
    for _, prov in ipairs(sortedProviders) do
      table.insert(cards, buildProviderCard(prov))
    end
  else
    table.insert(cards, ui.column({
      padding = 40,
      align = "center",
      justify = "center",
      gap = 10,
    }, {
      ui.glyph({ name = "sparkles", size = 36, color = "on_surface_variant" }),
      ui.label({
        text = "No provider quotas detected",
        fontSize = 16,
        fontWeight = "bold",
        color = "on_surface",
      }),
      ui.label({
        text = "Run 'omp usage' or verify authenticated accounts.",
        fontSize = 13,
        color = "on_surface_variant",
      }),
    }))
  end

  table.insert(contentElements, ui.scroll({ flexGrow = 1 }, {
    ui.column({ gap = 14 }, cards),
  }))

  panel.render(ui.column({
    padding = 20,
    gap = 14,
    flexGrow = 1,
  }, contentElements))
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

function onOpen()
  panel.setWantsSecondTicks(true)
  report = noctalia.state.get("report")
  polling = noctalia.state.get("polling") == true
  lastError = noctalia.state.get("last_error")
  if not report then
    noctalia.state.set("command", { action = "refresh" })
  end
  render()
end

function onClose()
  panel.setWantsSecondTicks(false)
end

function update()
  -- Live second ticks
  render()
end

function onKey(chord, pressed)
  if not pressed then
    return
  end
  local lower = tostring(chord or ""):lower()
  if lower == "escape" then
    if type(panel.close) == "function" then
      panel.close()
    end
  elseif lower == "r" then
    noctalia.state.set("command", { action = "refresh" })
  end
end

function onConfigChanged()
  highUsageThreshold = tonumber(noctalia.getConfig("high_usage_threshold")) or 75
  render()
end

render()

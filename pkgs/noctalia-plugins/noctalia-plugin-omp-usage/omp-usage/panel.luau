--!nonstrict
-- OMP Usage dashboard panel: provider quota cards, ordered most urgent first
-- Clean typography, neutral background surfaces, zero colored-on-colored background fills

local shared = require("./shared.luau")

local report = noctalia.state.get("report")
local polling = noctalia.state.get("polling") == true
local lastError = noctalia.state.get("last_error")

local function formatExactReset(resetsAtMs)
  if not resetsAtMs or resetsAtMs <= 0 then
    return ""
  end
  local diffSec = math.floor((resetsAtMs - noctalia.nowMs()) / 1000)
  local resetSec = math.floor(resetsAtMs / 1000)

  if diffSec < 86400 then
    return "at " .. os.date("%H:%M", resetSec)
  else
    return os.date("%a %b %d at %H:%M", resetSec)
  end
end

local function formatUpdated(fetchedAtMs)
  if not fetchedAtMs or fetchedAtMs <= 0 then
    return ""
  end
  local diffMin = math.floor((noctalia.nowMs() - fetchedAtMs) / 60000)
  if diffMin < 1 then
    return "Synced just now"
  end
  return string.format("Synced %dm ago", diffMin)
end

-- Spacious 2-tier limit card with clean neutral backgrounds
local function buildLimitBlock(lim)
  local fraction = math.min(1.0, math.max(0.0, (lim.used or 0) / 100.0))
  local color = shared.severityColor(lim.severity)
  local countdownStr = shared.formatCountdown(lim.resetsAt)
  local exactTime = formatExactReset(lim.resetsAt)
  local isStrained = lim.severity ~= "ok"

  local percentStr = string.format("%.0f%%", lim.used)
  if lim.used > 0 and lim.used < 1 then
    percentStr = string.format("%.1f%%", lim.used)
  end

  local resetFull = "Resets in " .. countdownStr
  if exactTime ~= "" then
    resetFull = resetFull .. " (" .. exactTime .. ")"
  end

  local paceRow = nil
  if lim.exhaustsAt then
    local paceText = "On pace to run out in " .. shared.formatCountdown(lim.exhaustsAt)
    local paceExact = formatExactReset(lim.exhaustsAt)
    if paceExact ~= "" then
      paceText = paceText .. " (" .. paceExact .. "), before it resets"
    end
    paceRow = ui.row({ gap = 6, align = "center" }, {
      ui.glyph({ name = "trending-up", size = 14, color = "tertiary" }),
      ui.label({ text = paceText, fontSize = 13, fontWeight = "medium", color = "tertiary" }),
    })
  end

  local rows = {
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
      lim.severity == "exhausted" and ui.label({
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
  }
  if paceRow then
    table.insert(rows, paceRow)
  end

  return ui.column({
    padding = 12,
    radius = 10,
    fill = "surface_variant/0.20",
    gap = 8,
  }, rows)
end

local function buildProviderCard(prov)
  local isExhausted = prov.severity == "exhausted"
  local statusText = isExhausted and "EXHAUSTED" or (prov.severity == "high" and "HIGH" or "ACTIVE")
  local statusColor = prov.severity == "ok" and "on_surface_variant" or shared.severityColor(prov.severity)

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

  local credits = prov.resetCredits
  if credits then
    local text = string.format("%d saved reset%s", credits.count, credits.count == 1 and "" or "s")
    if credits.expiresAtMs then
      text = text .. " · expires in " .. shared.formatCountdown(credits.expiresAtMs)
    end
    table.insert(limitItems, 1, ui.row({ gap = 6, align = "center" }, {
      ui.glyph({ name = "ticket", size = 14, color = "secondary" }),
      ui.label({ text = text, fontSize = 13, fontWeight = "medium", color = "secondary" }),
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

-- "3 providers · 1 exhausted · 1 high", from the service's per-provider severity.
local function summaryText()
  local providers = (report and report.providers) or {}
  if #providers == 0 then
    return "No usage data"
  end
  local counts = { exhausted = 0, high = 0 }
  for _, prov in ipairs(providers) do
    if counts[prov.severity] then
      counts[prov.severity] = counts[prov.severity] + 1
    end
  end
  local parts = { string.format("%d provider%s", #providers, #providers == 1 and "" or "s") }
  if counts.exhausted > 0 then
    table.insert(parts, counts.exhausted .. " exhausted")
  end
  if counts.high > 0 then
    table.insert(parts, counts.high .. " high")
  end
  if counts.exhausted == 0 and counts.high == 0 then
    table.insert(parts, "all within limits")
  end
  return table.concat(parts, " · ")
end

local function render()
  local contentElements = {}

  -- Top Header
  local syncText = polling and "Syncing..." or (report and formatUpdated(report.fetchedAtMs) or "")
  if not polling and report and lastError and lastError ~= "" then
    syncText = "Stale · " .. syncText
  end

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
        text = "OMP Usage",
        fontSize = 18,
        fontWeight = "bold",
        color = "on_surface",
      }),
      ui.label({
        text = summaryText(),
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
        panel.close()
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

  -- Accounts omp cannot report usage for (disabled credentials, missing reports)
  local issues = (report and report.issues) or {}
  if #issues > 0 then
    local issueRows = {}
    for _, issue in ipairs(issues) do
      local who = issue.account ~= "" and (issue.displayName .. " · " .. issue.account) or issue.displayName
      table.insert(issueRows, ui.row({ align = "center", gap = 8 }, {
        ui.glyph({ name = "user-exclamation", size = 16, color = "tertiary" }),
        ui.label({ text = who, fontSize = 13, fontWeight = "bold", color = "on_surface" }),
        ui.spacer({ flexGrow = 1 }),
        ui.label({ text = issue.detail, fontSize = 12, color = "tertiary", maxLines = 1 }),
      }))
    end
    table.insert(contentElements, ui.column({
      padding = 10,
      radius = 10,
      fill = "tertiary/0.10",
      gap = 6,
    }, issueRows))
  end

  -- Provider cards, already ordered most urgent first by the service
  local providers = (report and report.providers) or {}

  local cards = {}
  if #providers > 0 then
    for _, prov in ipairs(providers) do
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
    panel.close()
  elseif lower == "r" then
    noctalia.state.set("command", { action = "refresh" })
  end
end

render()

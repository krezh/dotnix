--!nonstrict
-- OMP Usage background service
-- Polls `omp usage --json` and publishes a display-ready report to noctalia.state:
-- severity, alert ordering and provider ordering are decided here, not in the views.

local shared = require("./shared.luau")

local COMMAND = "omp usage --json"

local PROVIDERS = {
  ["openai-codex"] = { name = "OpenAI Codex", glyph = "brand-openai" },
  ["google-antigravity"] = { name = "Google Antigravity", glyph = "brand-google" },
  ["github-copilot"] = { name = "GitHub Copilot", glyph = "brand-github" },
  anthropic = { name = "Anthropic", glyph = "sparkles" },
  openrouter = { name = "OpenRouter", glyph = "route" },
  deepseek = { name = "DeepSeek" },
  minimax = { name = "MiniMax" },
  zai = { name = "Zhipu AI (GLM)" },
}

local SEVERITY_RANK = { ok = 0, high = 1, exhausted = 2 }

local inFlight = false
local pendingRefresh = false
-- Last successfully decoded `omp` output, re-processed when settings change.
local lastDecoded = nil

local function intervalMs()
  local minutes = math.max(1, tonumber(noctalia.getConfig("refresh_minutes")) or 5)
  return math.floor(minutes * 60 * 1000)
end

local function providerInfo(id)
  local info = PROVIDERS[id] or {}
  local fallback = tostring(id or "Unknown")
  return info.name or (fallback:sub(1, 1):upper() .. fallback:sub(2)), info.glyph or "brain"
end

-- Appends the window name when the limit label does not already carry it,
-- e.g. "Gemini" -> "Gemini · 5 Hour", while "Claude 5 Hour" stays as is.
local function limitLabel(lim, window)
  local label = tostring(lim.label or window.label or "Limit")
  local windowLabel = window.label and tostring(window.label) or ""
  if windowLabel ~= "" and not string.find(label:lower(), windowLabel:lower(), 1, true) then
    label = label .. " · " .. windowLabel
  end
  return label
end

-- omp's own status is authoritative; the user threshold can only raise "ok" to "high".
local function limitSeverity(status, used, threshold)
  if status == "exhausted" then
    return "exhausted"
  elseif status ~= "ok" or used >= threshold then
    return "high"
  end
  return "ok"
end

-- Most urgent first: exhausted limits by soonest reset, then high limits by usage.
local function alertBefore(a, b)
  local ra, rb = SEVERITY_RANK[a.severity], SEVERITY_RANK[b.severity]
  if ra ~= rb then
    return ra > rb
  end
  if a.severity == "exhausted" and a.resetsAt and b.resetsAt and a.resetsAt ~= b.resetsAt then
    return a.resetsAt < b.resetsAt
  end
  return a.used > b.used
end

-- Pace projection only once the window has some history; earlier, a burst of a few
-- percent extrapolates to nonsense.
local PACE_MIN_ELAPSED_FRACTION = 0.1
local PACE_MIN_USED = 5

-- When a limit would hit 100% at its average burn rate so far, measured at `measuredAtMs`;
-- nil when it will not run out before it resets or there is too little history.
local function projectedExhaustion(used, resetsAt, durationMs, measuredAtMs)
  if not (resetsAt and durationMs and measuredAtMs) or durationMs <= 0 then
    return nil
  end
  local elapsedMs = durationMs - (resetsAt - measuredAtMs)
  if elapsedMs / durationMs < PACE_MIN_ELAPSED_FRACTION or used < PACE_MIN_USED or used >= 100 then
    return nil
  end
  local exhaustsAt = math.floor(measuredAtMs + (100 - used) * elapsedMs / used)
  if exhaustsAt >= resetsAt then
    return nil
  end
  return exhaustsAt
end

-- Unix milliseconds for an ISO-8601 UTC timestamp ("2026-10-22T16:00:00.000Z"); nil otherwise.
local function parseIsoUtcMs(iso)
  local y, mo, d, h, mi, s = tostring(iso or ""):match("^(%d+)-(%d+)-(%d+)T(%d+):(%d+):(%d+)")
  if not y then
    return nil
  end
  y, mo, d = tonumber(y), tonumber(mo), tonumber(d)
  -- Days since the epoch from a proleptic Gregorian date (Howard Hinnant's days_from_civil).
  if mo <= 2 then
    y = y - 1
  end
  local era = math.floor(y / 400)
  local yoe = y - era * 400
  local doy = math.floor((153 * (mo + (mo > 2 and -3 or 9)) + 2) / 5) + d - 1
  local doe = yoe * 365 + math.floor(yoe / 4) - math.floor(yoe / 100) + doy
  local days = era * 146097 + doe - 719468
  return ((days * 24 + tonumber(h)) * 60 + tonumber(mi)) * 60000 + tonumber(s) * 1000
end

-- Saved rate-limit resets: count plus the soonest expiry; nil when none are available.
local function resetCredits(raw)
  local count = type(raw) == "table" and tonumber(raw.availableCount) or 0
  if count <= 0 then
    return nil
  end
  local expiresAtMs = nil
  for _, credit in ipairs(type(raw.credits) == "table" and raw.credits or {}) do
    if type(credit) == "table" and credit.status == "available" then
      local expires = parseIsoUtcMs(credit.expiresAt)
      if expires and (expiresAtMs == nil or expires < expiresAtMs) then
        expiresAtMs = expires
      end
    end
  end
  return { count = count, expiresAtMs = expiresAtMs }
end

-- Accounts omp knows about but cannot report usage for: disabled credentials and
-- authenticated accounts with no usage report.
local function accountIssues(raw)
  local issues = {}
  local function add(entry, detail)
    if type(entry) == "table" and entry.provider then
      local displayName = providerInfo(entry.provider)
      table.insert(issues, {
        displayName = displayName,
        account = tostring(entry.email or entry.orgName or entry.type or ""),
        detail = detail,
      })
    end
  end
  for _, entry in ipairs(type(raw.disabledCredentials) == "table" and raw.disabledCredentials or {}) do
    add(entry, "Disabled: " .. tostring(entry.cause or "unknown cause"))
  end
  for _, entry in ipairs(type(raw.accountsWithoutUsage) == "table" and raw.accountsWithoutUsage or {}) do
    add(entry, "No usage reported")
  end
  return issues
end

local function processReport(raw, settings)
  if type(raw) ~= "table" or type(raw.reports) ~= "table" then
    return nil
  end

  local providers = {}
  local alerts = {}
  -- Oldest per-provider fetch time: the report is only as fresh as its stalest entry.
  local fetchedAtMs = nil

  for _, item in ipairs(raw.reports) do
    if type(item) == "table" and item.provider then
      local meta = item.metadata or {}
      local displayName, glyph = providerInfo(item.provider)
      local provObj = {
        id = item.provider,
        displayName = displayName,
        glyph = glyph,
        email = tostring(meta.email or ""),
        planType = tostring(meta.planType or ""),
        limits = {},
        severity = "ok",
        maxUsed = 0,
        resetCredits = resetCredits(item.resetCredits),
      }
      -- Limits drawing from one shared pool are reported once per member; keep the first.
      local seenGroups = {}

      local itemFetchedAt = tonumber(item.fetchedAt)
      if itemFetchedAt and (fetchedAtMs == nil or itemFetchedAt < fetchedAtMs) then
        fetchedAtMs = itemFetchedAt
      end

      for _, lim in ipairs(type(item.limits) == "table" and item.limits or {}) do
        local group = type(lim) == "table" and type(lim.scope) == "table" and lim.scope.sharedGroup or nil
        if type(lim) == "table" and not (group and seenGroups[group]) then
          if group then
            seenGroups[group] = true
          end

          local amount = lim.amount or {}
          local window = lim.window or {}
          local used = tonumber(amount.used) or 0
          local status = tostring(lim.status or "ok"):lower()
          local resetsAt = tonumber(window.resetsAt)
          local durationMs = tonumber(window.durationMs)
          local severity = limitSeverity(status, used, settings.threshold)
          local exhaustsAt = severity ~= "exhausted"
            and projectedExhaustion(used, resetsAt, durationMs, itemFetchedAt)
            or nil
          if exhaustsAt and settings.paceAlerts and severity == "ok" then
            severity = "high"
          end

          local limitObj = {
            key = string.format("%s|%s|%s", item.provider, provObj.email, tostring(lim.id or lim.label)),
            label = limitLabel(lim, window),
            used = used,
            remaining = tonumber(amount.remaining) or (100 - used),
            resetsAt = resetsAt,
            exhaustsAt = exhaustsAt,
            severity = severity,
            displayName = displayName,
            glyph = glyph,
          }

          table.insert(provObj.limits, limitObj)
          if limitObj.severity ~= "ok" then
            table.insert(alerts, limitObj)
          end
          if SEVERITY_RANK[limitObj.severity] > SEVERITY_RANK[provObj.severity] then
            provObj.severity = limitObj.severity
          end
          provObj.maxUsed = math.max(provObj.maxUsed, used)
        end
      end

      -- The limit that best represents this provider in compact views.
      local ranked = table.clone(provObj.limits)
      table.sort(ranked, alertBefore)
      provObj.headline = ranked[1]

      table.insert(providers, provObj)
    end
  end

  table.sort(alerts, alertBefore)
  table.sort(providers, function(a, b)
    local ra, rb = SEVERITY_RANK[a.severity], SEVERITY_RANK[b.severity]
    if ra ~= rb then
      return ra > rb
    end
    return a.maxUsed > b.maxUsed
  end)

  return {
    providers = providers,
    alerts = alerts,
    issues = accountIssues(raw),
    fetchedAtMs = fetchedAtMs or tonumber(raw.generatedAt) or noctalia.nowMs(),
  }
end

-- Severity per limit key from the previous published report; nil before the first one.
local previousSeverity = nil

-- Records each limit's severity as the new baseline; with `announce`, first notifies
-- every limit whose severity changed since the previous baseline.
local function trackSeverities(report, announce)
  local current = {}
  for _, prov in ipairs(report.providers) do
    for _, lim in ipairs(prov.limits) do
      current[lim.key] = lim.severity
      local before = previousSeverity and previousSeverity[lim.key]
      if announce and before and before ~= lim.severity then
        local title = string.format("%s · %s", lim.displayName, lim.label)
        if lim.severity == "exhausted" then
          noctalia.notifyError(title .. " exhausted", "Resets in " .. shared.formatCountdown(lim.resetsAt))
        elseif before == "exhausted" then
          noctalia.notify(title .. " available again", string.format("%.0f%% used", lim.used))
        elseif lim.severity == "high" then
          local body = lim.exhaustsAt
            and ("On pace to run out in " .. shared.formatCountdown(lim.exhaustsAt))
            or string.format("%.0f%% used", lim.used)
          noctalia.notify(title .. " running high", body)
        end
      end
    end
  end
  previousSeverity = current
end

-- Re-derives the published report from the last `omp` output. `fromPoll` is false for
-- settings changes, which must not raise notifications for unchanged usage.
local function publish(fromPoll)
  local report = processReport(lastDecoded, {
    threshold = tonumber(noctalia.getConfig("high_usage_threshold")) or 75,
    paceAlerts = noctalia.getConfig("pace_alerts") ~= false,
  })
  if not report then
    return false
  end
  trackSeverities(report, fromPoll and noctalia.getConfig("notifications") ~= false)
  noctalia.state.set("report", report)
  return true
end

-- `omp` runs through wrappers that print unrelated notices on stderr (e.g. update nags),
-- so the last non-empty line is the one most likely to describe the failure.
local function commandError(result)
  if not result then
    return "could not launch omp"
  end
  if result.timedOut then
    return "omp usage timed out"
  end
  local lastLine = nil
  for line in tostring(result.stderr or ""):gmatch("[^\r\n]+") do
    if line:match("%S") then
      lastLine = line
    end
  end
  local code = tostring(result.exitCode or "?")
  if lastLine then
    return string.format("omp usage failed (exit %s): %s", code, lastLine)
  end
  return string.format("omp usage failed (exit %s)", code)
end

-- Requests arriving while a poll runs are coalesced into one follow-up poll.
local function refresh()
  if inFlight then
    pendingRefresh = true
    return
  end

  pendingRefresh = false
  inFlight = true
  noctalia.state.set("polling", true)

  local launched = noctalia.runAsync(COMMAND, function(result)
    inFlight = false
    noctalia.state.set("polling", false)

    if result and result.exitCode == 0 and type(result.stdout) == "string" then
      local decoded = noctalia.json.decode(result.stdout)
      if type(decoded) == "table" then
        lastDecoded = decoded
        noctalia.state.set("last_error", (not publish(true)) and "Failed to parse usage data structure" or nil)
      else
        noctalia.state.set("last_error", "Invalid JSON from omp usage")
      end
    else
      noctalia.state.set("last_error", commandError(result))
    end

    if pendingRefresh then
      refresh()
    end
  end, 20000)

  if not launched then
    inFlight = false
    noctalia.state.set("polling", false)
    noctalia.state.set("last_error", commandError(nil))
  end
end

noctalia.state.watch("command", function(cmd)
  if type(cmd) == "table" and cmd.action == "refresh" then
    refresh()
  end
end)

function onIpc(event, _payload)
  if event == "refresh" then
    refresh()
  end
end

function update()
  if not inFlight then
    refresh()
  end
end

function onConfigChanged()
  noctalia.setUpdateInterval(intervalMs())
  if lastDecoded then
    publish(false)
  end
end

noctalia.setUpdateInterval(intervalMs())
refresh()

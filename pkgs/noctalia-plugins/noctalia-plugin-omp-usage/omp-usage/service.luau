--!nonstrict
-- OMP Usage background service
-- Periodically polls `omp usage --json` and publishes formatted report to noctalia.state

local refreshMinutes = tonumber(noctalia.getConfig("refresh_minutes")) or 5
if refreshMinutes < 1 then
  refreshMinutes = 1
end

local inFlight = false
local pendingRefresh = false
local lastStart = 0
local MIN_GAP_MS = 3000

local COMMAND = "omp usage --json"

local function intervalMs()
  local minutes = tonumber(noctalia.getConfig("refresh_minutes")) or 5
  if minutes < 1 then
    minutes = 1
  end
  return math.floor(minutes * 60 * 1000)
end

local function providerDisplayName(id)
  if id == "openai-codex" then
    return "OpenAI Codex"
  elseif id == "google-antigravity" then
    return "Google Antigravity"
  elseif id == "anthropic" then
    return "Anthropic"
  elseif id == "openrouter" then
    return "OpenRouter"
  elseif id == "github-copilot" then
    return "GitHub Copilot"
  elseif id == "deepseek" then
    return "DeepSeek"
  elseif id == "minimax" then
    return "MiniMax"
  elseif id == "zai" then
    return "Zhipu AI (GLM)"
  end
  -- Capitalize default
  local formatted = tostring(id or "Unknown")
  return formatted:sub(1, 1):upper() .. formatted:sub(2)
end

local function providerGlyph(id)
  if id == "openai-codex" then
    return "brand-openai"
  elseif id == "google-antigravity" then
    return "brand-google"
  elseif id == "github-copilot" then
    return "brand-github"
  elseif id == "anthropic" then
    return "sparkles"
  elseif id == "openrouter" then
    return "route"
  end
  return "brain"
end

local function processReport(raw)
  if type(raw) ~= "table" or type(raw.reports) ~= "table" then
    return nil
  end

  local providers = {}
  local mostCritical = nil
  local exhaustedCount = 0

  for _, item in ipairs(raw.reports) do
    if type(item) == "table" and item.provider then
      local meta = item.metadata or {}
      local pLimits = {}

      if type(item.limits) == "table" then
        for _, lim in ipairs(item.limits) do
          if type(lim) == "table" then
            local amount = lim.amount or {}
            local window = lim.window or {}
            local used = tonumber(amount.used) or 0
            local remaining = tonumber(amount.remaining) or (100 - used)
            local resetsAt = tonumber(window.resetsAt) or nil
            local durationMs = tonumber(window.durationMs) or nil
            local status = tostring(lim.status or "ok"):lower()

            if status == "exhausted" or used >= 99 then
              exhaustedCount = exhaustedCount + 1
            end

            local limitObj = {
              id = tostring(lim.id or ""),
              label = tostring(lim.label or window.label or "Limit"),
              used = used,
              remaining = remaining,
              resetsAt = resetsAt,
              durationMs = durationMs,
              status = status,
              provider = item.provider,
              displayName = providerDisplayName(item.provider),
              glyph = providerGlyph(item.provider),
            }

            table.insert(pLimits, limitObj)

            -- Evaluate if this limit is the most critical to highlight on the bar
            if mostCritical == nil then
              mostCritical = limitObj
            else
              local currExhausted = (limitObj.status == "exhausted" or limitObj.used >= 95)
              local prevExhausted = (mostCritical.status == "exhausted" or mostCritical.used >= 95)

              if currExhausted and not prevExhausted then
                mostCritical = limitObj
              elseif currExhausted and prevExhausted then
                -- Whichever resets soonest
                if limitObj.resetsAt and mostCritical.resetsAt then
                  if limitObj.resetsAt < mostCritical.resetsAt then
                    mostCritical = limitObj
                  end
                end
              elseif not currExhausted and not prevExhausted then
                -- Whichever has higher usage
                if limitObj.used > mostCritical.used then
                  mostCritical = limitObj
                end
              end
            end
          end
        end
      end

      local provObj = {
        id = item.provider,
        displayName = providerDisplayName(item.provider),
        glyph = providerGlyph(item.provider),
        email = tostring(meta.email or ""),
        planType = tostring(meta.planType or ""),
        limits = pLimits,
      }
      table.insert(providers, provObj)
    end
  end

  return {
    providers = providers,
    mostCritical = mostCritical,
    exhaustedCount = exhaustedCount,
    fetchedAt = os.time(),
  }
end

local function refresh()
  if inFlight then
    pendingRefresh = true
    return
  end

  local now = (noctalia.nowMs and noctalia.nowMs()) or (os.time() * 1000)
  if MIN_GAP_MS - (now - lastStart) > 0 then
    pendingRefresh = true
    return
  end

  lastStart = now
  pendingRefresh = false
  inFlight = true
  noctalia.state.set("polling", true)

  noctalia.runAsync(COMMAND, function(result)
    inFlight = false
    noctalia.state.set("polling", false)

    if result and result.exitCode == 0 and type(result.stdout) == "string" then
      local ok, decoded = pcall(noctalia.json.decode, result.stdout)
      if ok and type(decoded) == "table" then
        local report = processReport(decoded)
        if report then
          noctalia.state.set("report", report)
          noctalia.state.set("last_error", nil)
        else
          noctalia.state.set("last_error", "Failed to parse usage data structure")
        end
      else
        noctalia.state.set("last_error", "Invalid JSON from omp usage")
      end
    else
      local err = (result and result.stderr) or "omp command failed"
      noctalia.state.set("last_error", tostring(err))
    end

    if pendingRefresh then
      refresh()
    end
  end, 20000)
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
  if inFlight then
    return
  end
  refresh()
end

function onConfigChanged()
  noctalia.setUpdateInterval(intervalMs())
  refresh()
end

noctalia.setUpdateInterval(intervalMs())
refresh()

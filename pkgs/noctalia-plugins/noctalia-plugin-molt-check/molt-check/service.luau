--!nonstrict
-- Polls `molt check --json` and publishes the result to noctalia.state for the bar widget.
-- The last report and attempt time persist in the plugin data dir, so automatic checks run at
-- most once per refresh interval, even across restarts; a manual refresh always runs.

local COMMAND = { "molt", "check", "--json" }
-- `molt check` fetches the upstream; allow slow networks before giving up.
local TIMEOUT_MS = 120000
-- How often to test whether the refresh interval has elapsed.
local TICK_MS = 60000

local cachePath = nil
do
  local dir = noctalia.pluginDataDir()
  if dir then
    cachePath = dir .. "/cache.json"
  end
end

local inFlight = false
local pendingRefresh = false
-- Unix seconds of the last started check, successful or not.
local attemptedAt = 0
local report = nil

local function intervalSeconds()
  local minutes = tonumber(noctalia.getConfig("refresh_minutes")) or 15
  if minutes < 1 then
    minutes = 1
  end
  return math.floor(minutes * 60)
end

local function loadCache()
  if not cachePath then
    return
  end
  local raw = noctalia.readFile(cachePath)
  if not raw then
    return
  end
  local ok, decoded = pcall(noctalia.json.decode, raw)
  if not ok or type(decoded) ~= "table" then
    return
  end
  attemptedAt = tonumber(decoded.attemptedAt) or 0
  if type(decoded.report) == "table" and tonumber(decoded.report.incoming) then
    report = decoded.report
    noctalia.state.set("report", report)
  end
end

local function saveCache()
  if not cachePath then
    return
  end
  noctalia.writeFile(cachePath, noctalia.json.encode({
    attemptedAt = attemptedAt,
    report = report,
  }))
end

local function refresh()
  if inFlight then
    pendingRefresh = true
    return
  end
  inFlight = true
  pendingRefresh = false
  attemptedAt = os.time()
  noctalia.state.set("polling", true)

  local launched = noctalia.runAsync(COMMAND, function(result)
    inFlight = false
    noctalia.state.set("polling", false)

    if result and result.exitCode == 0 and type(result.stdout) == "string" then
      local ok, decoded = pcall(noctalia.json.decode, result.stdout)
      if ok and type(decoded) == "table" and tonumber(decoded.incoming) then
        report = {
          vcs = tostring(decoded.vcs or ""),
          upstream = tostring(decoded.upstream or ""),
          incoming = tonumber(decoded.incoming) or 0,
          ["local"] = tonumber(decoded["local"]) or 0,
          checkedAt = os.time(),
        }
        noctalia.state.set("report", report)
        noctalia.state.set("error", nil)
      else
        noctalia.state.set("error", "invalid JSON from molt check")
      end
    elseif result and result.timedOut then
      noctalia.state.set("error", "molt check timed out")
    else
      local stderr = result and result.stderr or ""
      if stderr == "" then
        stderr = "molt check failed"
      end
      noctalia.state.set("error", stderr)
    end
    saveCache()

    if pendingRefresh then
      refresh()
    end
  end, TIMEOUT_MS)

  if not launched then
    inFlight = false
    noctalia.state.set("polling", false)
    noctalia.state.set("error", "could not launch molt")
    saveCache()
  end
end

local function refreshIfDue()
  if inFlight then
    return
  end
  if os.time() - attemptedAt >= intervalSeconds() then
    refresh()
  end
end

function onIpc(event, _payload)
  if event == "refresh" then
    refresh()
  end
end

function update()
  refreshIfDue()
end

function onConfigChanged()
  refreshIfDue()
end

loadCache()
noctalia.setUpdateInterval(TICK_MS)
refreshIfDue()

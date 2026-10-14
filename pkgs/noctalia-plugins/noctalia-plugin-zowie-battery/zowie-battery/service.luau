--!nonstrict
-- Polls `zowie-batt --json` and publishes the reading to noctalia.state for the bar widget.
-- `absent` is set while zowie-batt is not installed or reports no receiver or wired mouse
-- (exit status 2), so the widget can hide on machines and moments without the mouse.

local COMMAND = { "zowie-batt", "--json" }
-- zowie-batt waits up to three 500 ms reply windows; leave headroom for a busy receiver.
local TIMEOUT_MS = 10000
local EXIT_NO_DEVICE = 2

local inFlight = false
local pendingRefresh = false

local function intervalMs()
  local seconds = tonumber(noctalia.getConfig("refresh_seconds")) or 60
  if seconds < 10 then
    seconds = 10
  end
  return math.floor(seconds * 1000)
end

local function markAbsent()
  noctalia.state.set("reading", nil)
  noctalia.state.set("error", nil)
  noctalia.state.set("absent", true)
end

-- Failures keep the last reading so the bar still shows it, marked with the error.
local function markError(message)
  noctalia.state.set("error", message)
  noctalia.state.set("absent", false)
end

local function refresh()
  if inFlight then
    pendingRefresh = true
    return
  end
  if not noctalia.commandExists("zowie-batt") then
    markAbsent()
    return
  end
  inFlight = true
  pendingRefresh = false

  local launched = noctalia.runAsync(COMMAND, function(result)
    inFlight = false

    if result and result.exitCode == 0 and type(result.stdout) == "string" then
      local ok, decoded = pcall(noctalia.json.decode, result.stdout)
      if ok and type(decoded) == "table" and tonumber(decoded.percent) and tonumber(decoded.voltage_mv) then
        noctalia.state.set("reading", {
          percent = tonumber(decoded.percent),
          voltageMv = tonumber(decoded.voltage_mv),
          connection = tostring(decoded.connection or ""),
          -- JSON null when read through the receiver.
          power = type(decoded.power) == "string" and decoded.power or nil,
          checkedAt = os.time(),
        })
        noctalia.state.set("error", nil)
        noctalia.state.set("absent", false)
      else
        markError("invalid JSON from zowie-batt")
      end
    elseif result and result.exitCode == EXIT_NO_DEVICE then
      markAbsent()
    elseif result and result.timedOut then
      markError("zowie-batt timed out")
    else
      local stderr = result and result.stderr or ""
      if stderr == "" then
        stderr = "zowie-batt failed"
      end
      markError(stderr)
    end

    if pendingRefresh then
      refresh()
    end
  end, TIMEOUT_MS)

  if not launched then
    inFlight = false
    markError("could not launch zowie-batt")
  end
end

function onIpc(event, _payload)
  if event == "refresh" then
    refresh()
  end
end

function update()
  refresh()
end

function onConfigChanged()
  noctalia.setUpdateInterval(intervalMs())
  refresh()
end

noctalia.setUpdateInterval(intervalMs())
refresh()

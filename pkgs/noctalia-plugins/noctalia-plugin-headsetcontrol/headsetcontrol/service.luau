--!nonstrict
-- Polls `headsetcontrol -o json` and publishes battery-capable headsets to noctalia.state for
-- the bar widget. `absent` is set while headsetcontrol is not installed or finds no headset
-- with a battery, so the widget can hide.

-- JSON output makes headsetcontrol query every readable status, battery included.
local COMMAND = { "headsetcontrol", "-o", "json" }
-- Wireless headsets can take a while to answer through their dongle.
local TIMEOUT_MS = 15000

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
  noctalia.state.set("headsets", nil)
  noctalia.state.set("error", nil)
  noctalia.state.set("absent", true)
end

-- Failures keep the last headsets so the bar still shows them, marked with the error.
local function markError(message)
  noctalia.state.set("error", message)
  noctalia.state.set("absent", false)
end

local function optionalNumber(value)
  return type(value) == "number" and value or nil
end

-- Keeps the battery and error fields the bar shows; drops devices without a battery.
local function parseHeadsets(report)
  local headsets = {}
  for _, dev in ipairs(type(report.devices) == "table" and report.devices or {}) do
    local battery = type(dev) == "table" and dev.battery
    if type(battery) == "table" then
      local errors = {}
      if type(dev.errors) == "table" then
        for source, message in pairs(dev.errors) do
          table.insert(errors, string.format("%s: %s", tostring(source), tostring(message)))
        end
        table.sort(errors)
      end
      table.insert(headsets, {
        name = tostring(dev.device or dev.product or "Headset"),
        status = tostring(battery.status or "BATTERY_UNAVAILABLE"),
        level = tonumber(battery.level) or -1,
        voltageMv = optionalNumber(battery.voltage_mv),
        timeToFullMin = optionalNumber(battery.time_to_full_min),
        timeToEmptyMin = optionalNumber(battery.time_to_empty_min),
        errors = errors,
      })
    end
  end
  return headsets
end

local function refresh()
  if inFlight then
    pendingRefresh = true
    return
  end
  if not noctalia.commandExists("headsetcontrol") then
    markAbsent()
    return
  end
  inFlight = true
  pendingRefresh = false

  local launched = noctalia.runAsync(COMMAND, function(result)
    inFlight = false

    -- headsetcontrol exits 1 with `device_count: 0` when nothing is connected, so the JSON
    -- decides presence, not the exit status.
    local ok, decoded = false, nil
    if result and type(result.stdout) == "string" and result.stdout ~= "" then
      ok, decoded = pcall(noctalia.json.decode, result.stdout)
    end

    if ok and type(decoded) == "table" and tonumber(decoded.device_count) then
      local headsets = parseHeadsets(decoded)
      if #headsets == 0 then
        markAbsent()
      else
        noctalia.state.set("headsets", { list = headsets, checkedAt = os.time() })
        noctalia.state.set("error", nil)
        noctalia.state.set("absent", false)
      end
    elseif result and result.timedOut then
      markError("headsetcontrol timed out")
    else
      local stderr = result and result.stderr or ""
      if stderr == "" then
        stderr = "invalid output from headsetcontrol"
      end
      markError(stderr)
    end

    if pendingRefresh then
      refresh()
    end
  end, TIMEOUT_MS)

  if not launched then
    inFlight = false
    markError("could not launch headsetcontrol")
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

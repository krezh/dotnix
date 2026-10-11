--!nonstrict
-- Helpers shared by the bar widget and panel. Severity is computed by the service;
-- these only present it.

local M = {}

-- Theme color role per limit/provider severity ("ok" | "high" | "exhausted").
function M.severityColor(severity)
  if severity == "exhausted" then
    return "error"
  elseif severity == "high" then
    return "tertiary"
  end
  return "primary"
end

-- Compact time until `resetsAtMs`: "45s", "12m", "3h 5m", "2d 4h"; "--" when unknown.
function M.formatCountdown(resetsAtMs)
  if not resetsAtMs or resetsAtMs <= 0 then
    return "--"
  end
  local diffMs = resetsAtMs - noctalia.nowMs()
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
  if hours < 24 then
    return hours .. "h " .. (min % 60) .. "m"
  end

  return math.floor(hours / 24) .. "d " .. (hours % 24) .. "h"
end

return M

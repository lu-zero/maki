-- Civil-time peak window. Z.AI coding plan bills 14:00-18:00 UTC+8
-- Monday to Friday at the full credit rate; every other hour is off-peak.

local M = {}

local SUNDAY = 1
local SATURDAY = 7
local SECS_PER_HOUR = 3600
local MS_PER_SEC = 1000
-- A week of hourly steps is enough to reach the next weekday window.
local MAX_HOUR_STEPS = 8 * 24

function M.civil(unix, utc_offset)
  return os.date("!*t", unix + utc_offset * SECS_PER_HOUR)
end

function M.in_peak(civil, start_hour, end_hour, weekdays_only)
  if weekdays_only and (civil.wday == SUNDAY or civil.wday == SATURDAY) then
    return false
  end
  local hour = civil.hour
  if start_hour < end_hour then
    return hour >= start_hour and hour < end_hour
  end
  if start_hour > end_hour then
    return hour >= start_hour or hour < end_hour
  end
  return false
end

function M.ms_until_boundary(unix, start_hour, end_hour, utc_offset, weekdays_only)
  local function peaked(at)
    return M.in_peak(M.civil(at, utc_offset), start_hour, end_hour, weekdays_only)
  end
  local now_peak = peaked(unix)
  local civil = M.civil(unix, utc_offset)
  local into_hour = civil.min * 60 + civil.sec
  local wait = SECS_PER_HOUR - into_hour
  if wait == 0 then
    wait = SECS_PER_HOUR
  end
  local t = unix + wait
  for _ = 1, MAX_HOUR_STEPS do
    if peaked(t) ~= now_peak then
      return (t - unix) * MS_PER_SEC
    end
    t = t + SECS_PER_HOUR
  end
  return SECS_PER_HOUR * MS_PER_SEC
end

function M.provider_of(spec)
  return (spec or ""):match("^([^/]+)")
end

function M.watches(spec, providers)
  if providers == nil or providers == "" then
    return true
  end
  local slug = M.provider_of(spec)
  if not slug then
    return false
  end
  for raw in string.gmatch(providers, "[^,]+") do
    if raw:match("^%s*(.-)%s*$") == slug then
      return true
    end
  end
  return false
end

return M

local window = require("peak_window")
local th = require("maki.test_helpers")

local case = th.case
local eq = th.eq

-- Friday 2026-09-11 06:00 UTC = 14:00 UTC+8, start of the Z.AI window.
local FRI_1400 = 1789106400
-- Friday 2026-09-11 10:00 UTC = 18:00 UTC+8, first off-peak hour.
local FRI_1800 = 1789120800
-- Saturday 2026-09-12 06:00 UTC = 14:00 UTC+8.
local SAT_1400 = 1789192800
-- Monday 2026-09-14 06:00 UTC = 14:00 UTC+8.
local MON_1400 = 1789365600

local OFFSET = 8
local START = 14
local END = 18
local WEEKDAYS = true
local MS = 1000

local function peaked(unix)
  return window.in_peak(window.civil(unix, OFFSET), START, END, WEEKDAYS)
end

case("civil_shifts_utc_by_offset", function()
  local t = window.civil(FRI_1400, OFFSET)
  eq(t.hour, 14)
  eq(t.min, 0)
  eq(t.wday, 6)
end)

case("in_peak_at_window_start", function()
  eq(peaked(FRI_1400), true)
end)

case("in_peak_last_hour", function()
  eq(peaked(FRI_1800 - 1), true)
end)

case("off_peak_at_window_end", function()
  eq(peaked(FRI_1800), false)
end)

case("weekend_is_off_peak", function()
  eq(peaked(SAT_1400), false)
end)

case("weekend_ignored_when_weekdays_only_false", function()
  eq(window.in_peak(window.civil(SAT_1400, OFFSET), START, END, false), true)
end)

case("equal_hours_never_peak", function()
  eq(window.in_peak(window.civil(FRI_1400, OFFSET), 14, 14, false), false)
end)

case("wrapping_window", function()
  local t = window.civil(FRI_1400, OFFSET)
  eq(window.in_peak(t, 22, 2, false), false)
  local late = window.civil(FRI_1400 + 9 * 3600, OFFSET)
  eq(late.hour, 23)
  eq(window.in_peak(late, 22, 2, false), true)
end)

case("ms_until_end_from_window_start", function()
  eq(window.ms_until_boundary(FRI_1400, START, END, OFFSET, WEEKDAYS), 4 * 3600 * MS)
end)

case("ms_until_monday_from_friday_end", function()
  eq(window.ms_until_boundary(FRI_1800, START, END, OFFSET, WEEKDAYS), (MON_1400 - FRI_1800) * MS)
end)

case("provider_of_splits_spec", function()
  eq(window.provider_of("zai/glm-5-code"), "zai")
  eq(window.provider_of("anthropic/claude-opus-4-6"), "anthropic")
  eq(window.provider_of(""), nil)
end)

case("watches_default_list", function()
  eq(window.watches("zai/glm-5.1", "zai"), true)
  eq(window.watches("anthropic/claude-opus-4-6", "zai"), false)
  eq(window.watches("zai/glm-5.1", ""), true)
  eq(window.watches("zai/glm-5.1", "zai, deepseek"), true)
end)

th.report()

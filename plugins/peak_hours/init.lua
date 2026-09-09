-- Stops Z.AI turns during the coding-plan peak window so quota is not
-- spent at the full credit rate. Maki has no model failover: 429 retries
-- and key rotation stay on the same provider, so this plugin cancels, or
-- switches the focused session to `fallback` when that option is set.

local window = require("peak_window")

local opts = maki.api.register_options({
  providers = {
    default = "zai",
    desc = "Comma-separated provider slugs to watch. Empty watches every provider.",
  },
  fallback = {
    default = "",
    desc = "Model spec to switch the focused session to during peak. Empty cancels instead.",
  },
  start_hour = {
    default = 14,
    min = 0,
    desc = "Peak window start hour, inclusive, in the offset below. Z.AI coding plan is 14.",
  },
  end_hour = {
    default = 18,
    min = 0,
    desc = "Peak window end hour, exclusive. Z.AI coding plan is 18.",
  },
  utc_offset = {
    default = 8,
    min = -12,
    desc = "Hours east of UTC for the window. Z.AI publishes peak in UTC+8.",
  },
  weekdays_only = {
    default = true,
    desc = "If true, Saturday and Sunday are off-peak, matching the Z.AI coding plan.",
  },
})

local TITLE = "peak hours"
local saved_spec = nil
local was_peak = false
local pending
local started = false

local function in_peak_now()
  return window.in_peak(window.civil(os.time(), opts.utc_offset), opts.start_hour, opts.end_hour, opts.weekdays_only)
end

local function window_label()
  return string.format("%02d:00-%02d:00 UTC%+d", opts.start_hour, opts.end_hour, opts.utc_offset)
end

local function watched(spec)
  return window.watches(spec, opts.providers)
end

local function stop_session(id)
  local snap, err = maki.session.read({ session = id })
  if err or not snap or not watched(snap.model) then
    return false
  end
  if snap.status == "idle" then
    return false
  end
  maki.session.cancel({ session = id })
  return true
end

local function stop_watched()
  local live = maki.session.live()
  if not live then
    return 0
  end
  local n = 0
  for _, session in ipairs(live) do
    if stop_session(session.id) then
      n = n + 1
    end
  end
  return n
end

local function switch_fallback()
  if opts.fallback == "" then
    return
  end
  local current = maki.model.get()
  if not current or not current.spec then
    return
  end
  if current.spec == opts.fallback then
    return
  end
  if not watched(current.spec) then
    return
  end
  saved_spec = current.spec
  maki.model.set(opts.fallback)
end

local function restore_fallback()
  if not saved_spec then
    return
  end
  maki.model.set(saved_spec)
  saved_spec = nil
end

local function enter_peak()
  switch_fallback()
  local n = stop_watched()
  local msg
  if n > 0 then
    msg = "Z.AI peak window (" .. window_label() .. "). Turns cancelled."
  elseif saved_spec then
    msg = "Z.AI peak window (" .. window_label() .. "). Switched to fallback model."
  end
  if msg then
    maki.notify(msg, "warn", { title = TITLE })
  end
end

local function leave_peak()
  restore_fallback()
  maki.notify("Peak window ended (" .. window_label() .. ").", "info", { title = TITLE })
end

local function tick()
  local peak = in_peak_now()
  if peak and not was_peak then
    enter_peak()
  elseif (not peak) and was_peak then
    leave_peak()
  elseif peak then
    stop_watched()
  end
  was_peak = peak
end

local function arm()
  if pending then
    pending:stop()
  end
  local delay = window.ms_until_boundary(os.time(), opts.start_hour, opts.end_hour, opts.utc_offset, opts.weekdays_only)
  pending = maki.defer_fn(function()
    tick()
    arm()
  end, math.max(delay, 1))
end

local function ensure_started()
  if started then
    return
  end
  started = true
  tick()
  arm()
end

-- Do not roundtrip to the UI during load: tests and cold start have a
-- sender with nobody draining it, so session.live would park forever.
-- SessionFocusChanged fires once the TUI is up.
maki.api.create_autocmd({ "SessionFocusChanged", "TurnStart", "SessionStatusChanged" }, {
  callback = function(ev)
    ensure_started()
    if ev.event ~= "TurnStart" or not in_peak_now() then
      return
    end
    local id = ev.data and ev.data.session_id
    if id and stop_session(id) then
      switch_fallback()
      maki.notify("Z.AI peak window (" .. window_label() .. "). Turn cancelled.", "warn", { title = TITLE })
    end
  end,
})

maki.api.register_command({
  name = "/peak",
  description = "Show whether the configured peak window is active",
  handler = function()
    ensure_started()
    local peak = in_peak_now()
    local civil = window.civil(os.time(), opts.utc_offset)
    local state = peak and "in peak window" or "off-peak"
    maki.notify(
      string.format("%s · %s · %02d:%02d", state, window_label(), civil.hour, civil.min),
      "info",
      { title = TITLE }
    )
  end,
})

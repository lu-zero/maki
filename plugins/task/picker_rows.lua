-- Row building for the /tasks picker in picker.lua: filtering, ordering and
-- sections, with no host calls and no globals. Every refresh throws the old
-- rows away and builds them again, so nothing here can drift from the host
-- list.

local ListPicker = require("maki.list_picker")

local RUNNING_SECTION = "Running"
local JOBS_SECTION = "Jobs"
local FINISHED_SECTION = "Finished"

local M = {}

-- An idle plugin task is waiting for its next prompt, not finished.
local function alive(status)
  return status == "working" or status == "idle"
end

-- An unnamed job is identified by the command it runs.
local function job_label(job)
  return job.name or job.command
end

-- The main chat comes first and has no status. The subagents follow, live
-- ones above finished ones so a long job never gets buried under the ones
-- that already returned. The session's running jobs sit between the two:
-- they are work in progress the picker can stop, whether or not this plugin
-- started them.
--
-- Within a task section, newest first (`tasks` comes in chat order, which is
-- spawn order), so the latest task is the one at hand. Jobs keep list order.
-- An exited job leaves the list: its owner already reported how it ended.
--
-- Returns { rows, sections }. A row carries a section header only when it opens
-- one, and `sections` counts what survived the filter.
function M.build(tasks, jobs, query)
  local words = ListPicker.split_words(query)
  local main, running, finished = nil, {}, {}
  for _, task in ipairs(tasks) do
    if ListPicker.matches(task.name, words) then
      if not task.status then
        main = task
      elseif alive(task.status) then
        running[#running + 1] = task
      else
        finished[#finished + 1] = task
      end
    end
  end

  local live_jobs = {}
  for _, job in ipairs(jobs or {}) do
    if job.status == "running" and ListPicker.matches(job_label(job), words) then
      live_jobs[#live_jobs + 1] = job
    end
  end

  local rows = {}
  if main then
    rows[#rows + 1] = { task = main }
  end
  -- Tasks read newest first, so their header goes on the last row drawn.
  local function append(header, items, key, newest_first)
    local from, to, step = 1, #items, 1
    if newest_first then
      from, to, step = #items, 1, -1
    end
    for i = from, to, step do
      rows[#rows + 1] = { [key] = items[i], section = i == from and header or nil }
    end
  end
  append(RUNNING_SECTION, running, "task", true)
  append(JOBS_SECTION, live_jobs, "job", false)
  append(FINISHED_SECTION, finished, "task", true)
  return {
    rows = rows,
    sections = { running = #running, jobs = #live_jobs, finished = #finished },
  }
end

function M.row_id(row)
  return row.task and row.task.id or row.job.id
end

function M.row_name(row)
  return row.task and row.task.name or job_label(row.job)
end

-- Only a finished subagent may leave the list. The main chat is the session
-- itself, dropping a live task would orphan its transcript, and a job's
-- process is not ours to delete. The host has the final say. This check only
-- decides which rows ask "are you sure".
function M.deletable(task)
  return task.status ~= nil and not alive(task.status)
end

-- Position of {id} among {rows}, or nil. The selection is kept as an id and
-- resolved here at render time, so a task moving between sections never drags
-- the cursor with it.
function M.index_of(rows, id)
  for i, row in ipairs(rows) do
    if M.row_id(row) == id then
      return i
    end
  end
  return nil
end

return M

local helper = require("addon.tests.support")

-- Builds the window with a controllable timer queue and quest log, and counts
-- transcript redraws caused by the item, macro and quest events.
local function scenario()
    local env = helper.env()
    setfenv(1, env)
    helper.fixture("addon/tests/wow-ui-stub.lua", env)
    local timers = {}
    C_Timer = {
        After = function(delay, fn)
            timers[#timers + 1] = { delay = delay, fn = fn }
        end,
    }
    local quest = { title = "Quest A", level = 10 }
    C_QuestLog = {
        GetNumQuestLogEntries = function()
            return 1
        end,
        GetInfo = function()
            return { isHeader = false, questID = 42, title = quest.title, level = quest.level }
        end,
    }
    local writes = 0
    local stored = {}
    local questReferences = setmetatable({}, {
        __index = stored,
        __newindex = function(_, key, value)
            writes = writes + 1
            stored[key] = value
        end,
    })
    ns = helper.namespace({
        db = { sidebar = true, showTools = true, drafts = {}, questReferences = questReferences },
        active = "chat",
        connected = true,
    })
    ns.chats = { { id = "chat", title = "Events", status = "idle", messages = {} } }
    function ns.Chat.Current()
        return ns.chats[1]
    end
    helper.addon("UI/Window.lua", env)
    ns.UI.BuildUI()

    local events
    for _, widget in ipairs(UI_TEST.widgets) do
        if not widget.name and widget.scripts.OnEvent then
            assert(not events, "expected one item and quest event frame")
            events = widget
        end
    end
    assert(events, "the window must listen for item and quest events")

    local state = { redraws = 0, quest = quest }
    function ns.UI.Refresh()
        state.redraws = state.redraws + 1
    end
    for index = #timers, 1, -1 do
        timers[index] = nil
    end
    writes = 0

    function state.fire(event, ...)
        events.scripts.OnEvent(events, event, ...)
    end
    function state.timers()
        return #timers
    end
    function state.delays()
        local delays = {}
        for _, timer in ipairs(timers) do
            delays[#delays + 1] = timer.delay
        end
        table.sort(delays)
        return delays
    end
    function state.flush()
        local due = timers
        timers = {}
        for _, timer in ipairs(due) do
            timer.fn()
        end
    end
    function state.writes()
        return writes
    end
    function state.revision()
        return ns.itemRevision
    end
    return state
end

describe("item, macro and quest events", function()
    it("redraw a burst of item data once, after one timer", function()
        local s = scenario()
        local revision = s.revision()
        for id = 1, 50 do
            s.fire("GET_ITEM_INFO_RECEIVED", id, true)
        end
        s.fire("UPDATE_MACROS")
        assert.same({ 0.2 }, s.delays())
        assert.equal(0, s.redraws)
        assert.equal(revision, s.revision())
        s.flush()
        assert.equal(1, s.redraws)
        assert.equal(revision + 1, s.revision())
        s.fire("GET_ITEM_INFO_RECEIVED", 51, true)
        assert.equal(1, s.timers(), "a later burst arms a new timer")
    end)

    it("leave unchanged quests alone and redraw a real change once", function()
        local s = scenario()
        for _ = 1, 10 do
            s.fire("QUEST_LOG_UPDATE")
        end
        assert.same({ 0.4 }, s.delays())
        s.flush()
        assert.equal(0, s.writes(), "BuildUI already stored the quest")
        assert.equal(0, s.redraws)

        s.quest.level = 11
        for _ = 1, 10 do
            s.fire("QUEST_LOG_UPDATE")
        end
        assert.equal(1, s.timers())
        s.flush()
        assert.equal(1, s.writes())
        assert.equal(1, s.redraws)
    end)

    it("batch item and quest events on separate timers", function()
        local s = scenario()
        s.quest.title = "Quest B"
        s.fire("GET_ITEM_INFO_RECEIVED", 7, true)
        s.fire("QUEST_LOG_UPDATE")
        s.fire("GET_ITEM_INFO_RECEIVED", 8, true)
        s.fire("QUEST_LOG_UPDATE")
        assert.same({ 0.2, 0.4 }, s.delays())
        s.flush()
        assert.equal(2, s.redraws)
    end)
end)

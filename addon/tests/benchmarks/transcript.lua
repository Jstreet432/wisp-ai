-- Offline CPU measurement of one transcript refresh, not WoW frame time: the
-- stub widgets do no font layout, so a real frame costs more. It shows how a
-- refresh grows with the number of messages in the chat.
-- Run from the repository root with Lua 5.1: lua addon/tests/benchmarks/transcript.lua
package.path = "./?.lua;" .. package.path
local helper = require("addon.tests.support")

local REPLY = table.concat({
    "## Gear for [[ref:robe]]",
    "",
    "Your **chest** slot is the weakest piece. Upgrade it first, then *trinkets*.",
    "",
    "- Robe from the dungeon boss, about 12% drop chance",
    "- Crafted option if you have `Tailoring` at 250",
    "- Quest reward from the zone you are in",
    "",
    "| Slot | Item | Source |",
    "|---|---|---|",
    "| Chest | [[ref:robe]] | Dungeon |",
    "| Trinket | Charm | Vendor |",
}, "\n")

local function chat(count)
    local messages = {}
    for index = 1, count do
        if index % 2 == 1 then
            messages[index] = { id = "m" .. index, role = "user", text = "Question " .. index }
        else
            messages[index] = {
                id = "m" .. index,
                role = "assistant",
                phase = "reply",
                text = REPLY,
                references = { robe = { kind = "item", id = 100, label = "Robe" } },
            }
        end
    end
    return { id = "chat", title = "Benchmark", status = "done", messages = messages }
end

local REPEAT = 20

local function median(values)
    table.sort(values)
    return values[math.floor(#values / 2) + 1] * 1000
end

local function measure(count)
    local env = helper.env()
    helper.fixture("addon/tests/wow-ui-stub.lua", env)
    env.ns = helper.namespace({
        db = { sidebar = true, showTools = true, drafts = {} },
        active = "chat",
        connected = true,
    })
    local ns = env.ns
    local current = chat(count)
    ns.chats = { current }
    function ns.Chat.Current()
        return current
    end
    helper.addon("UI/Window.lua", env)
    -- Parse every reply up front so refreshes measure rendering, not parsing.
    for _, message in ipairs(current.messages) do
        for _ = 1, 2000 do
            if not ns.Markdown.ReplyView.Project(message)._markdownPending then
                break
            end
            ns.Markdown.Cache.Step()
        end
    end
    local writes = env.UI_TEST.textWrites
    ns.UI.BuildUI()
    writes = env.UI_TEST.textWrites - writes

    -- os.clock is coarse on Windows, so each sample averages many refreshes.
    local function sample(prepare)
        local times = {}
        for _ = 1, 7 do
            local elapsed = 0
            for _ = 1, REPEAT do
                if prepare then
                    prepare()
                end
                local start = os.clock()
                ns.UI.Refresh()
                elapsed = elapsed + os.clock() - start
            end
            times[#times + 1] = elapsed / REPEAT
        end
        return median(times)
    end
    local unchanged = sample()
    local shown = 0
    for _, widget in ipairs(env.UI_TEST.widgets) do
        if rawget(widget, "replyView") and widget:IsShown() then
            shown = shown + 1
        end
    end
    local extra = 0
    local appended = sample(function()
        extra = extra + 1
        current.messages[#current.messages + 1] =
            { id = "extra" .. extra, role = "user", text = "One more question" }
    end)
    return writes, shown, unchanged, appended
end

print("messages\trows_shown\tfirst_text_writes\tunchanged_ms\tafter_append_ms")
for _, count in ipairs({ 10, 50, 100, 200, 400 }) do
    collectgarbage("collect")
    local writes, shown, unchanged, appended = measure(count)
    print(string.format("%d\t%d\t%d\t%.3f\t%.3f", count, shown, writes, unchanged, appended))
end

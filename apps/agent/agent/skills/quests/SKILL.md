---
name: quests
description: Choose the next quest or investigate quest objectives, prerequisites and follow-ups using the current log and verified locations.
---

Recommend one suitable active quest or turn-in from the fresh log, level and location. Explain the choice with verified facts and account for travel. A completed quest in another town is not automatically the fastest option; a quest name does not establish proximity.

When the player follows a Polestar guide or asks about their guide or step, read polestar__get_guide_step first and anchor the answer to that step; it is the route they chose. Explain or help with the step rather than proposing a different route unless asked.

When the player wants a route to follow (a quest chain, a gathering run, a list of errands) and polestar__create_guide is available, verify every quest, NPC, object, item and map point first, then build the guide with it instead of a long written list. Keep the returned id to update that guide when the plan changes.

Fetch questie__get_quest details only to resolve necessary objectives or entity IDs. Use bounded questie__get_quest_chain reads for prerequisites or follow-ups. Static requirements do not establish current eligibility. Look up locations when needed to distinguish candidates; avoid catalog/search chains for optional details. If an optional lookup fails, answer from the verified log.

--[[
  strategy.lua — Current AI Bot Strategy
  =======================================
  This file defines the bot's high-level decision-making priorities.
  It is updated by the post-run analyst after each death to reflect
  lessons learned. Newer entries (AUTO-PATCH blocks) take precedence
  over older entries.

  The AI reads this file before every action decision.
--]]

-- ── Priority Order ─────────────────────────────────────────────────────────────

-- The bot evaluates these conditions in order. The first matching
-- condition determines the action.

PRIORITY = {
  -- 1. Emergency healing
  {
    condition = "hp_percent < 0.25",
    action    = "use healing potion if available; else retreat north until clear",
    reason    = "Survival above all else.",
  },
  -- 2. Poisoned
  {
    condition = "status_effect == 'poisoned'",
    action    = "use antidote potion immediately; if none, retreat and wait for it to wear off",
    reason    = "Poison compounds; deal with it before it kills.",
  },
  -- 3. Troll nearby without fire
  {
    condition = "enemy_visible == 'troll' and not has_item('fire potion')",
    action    = "retreat — do NOT engage trolls without fire",
    reason    = "Trolls regenerate and deal very high damage.",
  },
  -- 4. Attack adjacent enemy
  {
    condition = "enemy_adjacent",
    action    = "attack; if enemy is skeleton use blunt weapon",
    reason    = "Melee enemies must be killed before they surround us.",
  },
  -- 5. Move toward visible enemy
  {
    condition = "enemy_visible and not enemy_adjacent",
    action    = "approach via corridor; never walk into open room with 3+ enemies",
    reason    = "Control engagement range.",
  },
  -- 6. Pick up priority item
  {
    condition = "item_on_floor and is_priority_item(item_on_floor)",
    action    = "pick up item",
    reason    = "Maintain resource advantage.",
  },
  -- 7. Explore
  {
    condition = "no_enemy_visible",
    action    = "explore unseen tiles; prefer corridors over open rooms",
    reason    = "Progress through dungeon.",
  },
  -- 8. Descend
  {
    condition = "stairs_found and hp_percent > 0.6 and has_food",
    action    = "descend to next level",
    reason    = "Keep pushing depth for XP and better loot.",
  },
}

-- ── Exploration Heuristics ─────────────────────────────────────────────────────

EXPLORATION = {
  prefer_corridors     = true,   -- Corridors limit enemy angles of attack.
  open_doors           = true,   -- Always open doors (may be loot behind them).
  avoid_open_rooms     = true,   -- Unless fully explored and cleared.
  hug_walls            = false,  -- Waste of time; go direct.
  backtrack_on_stuck   = true,   -- If 5 moves in same direction, try another route.
}

-- ── Resource Management ────────────────────────────────────────────────────────

RESOURCES = {
  healing_potion_threshold = 0.35,   -- Use healing potion at or below this HP%.
  food_eat_threshold       = 30,     -- Eat food when hunger below this value.
  descend_hp_threshold     = 0.60,   -- Don't descend below this HP%.
  min_food_before_descend  = 1,      -- Must have at least 1 food item to descend.
}

-- ── Combat Tactics ─────────────────────────────────────────────────────────────

COMBAT_TACTICS = {
  -- Corridor funnel: fight one enemy at a time by standing in a doorway.
  use_corridor_chokepoint = true,
  -- If 2+ enemies in melee, retreat one tile to reset to 1v1.
  retreat_on_surround     = true,
  -- Don't waste arrows on enemies that can be easily melee'd.
  conserve_ammo           = true,
  -- Use spells only when outnumbered or vs high-danger enemies.
  conserve_mana           = true,
}

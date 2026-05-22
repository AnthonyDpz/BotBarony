--[[
  death_patterns.lua — Known Death Patterns & Solutions
  ======================================================
  Records patterns of death that have occurred in past runs along with
  the corrective strategy. The post-run analyst appends to this file
  after each death.

  Format: each DEATH_PATTERN entry contains:
    - trigger:   conditions that led to death
    - cause:     what killed the bot
    - solution:  how to avoid this in future runs
    - run_count: how many times this pattern has been observed
--]]

DEATH_PATTERNS = {}

-- ── Pattern 001: Troll without fire ───────────────────────────────────────────
DEATH_PATTERNS[1] = {
  trigger   = "hp < 60% and engaged troll in open room",
  cause     = "Troll melee + regeneration",
  solution  = "NEVER engage a troll without a fire potion. Retreat immediately on sight. "
            .. "Circle around via a different corridor.",
  run_count = 0,   -- Will be incremented by the analyst.
}

-- ── Pattern 002: Surrounded in open room ──────────────────────────────────────
DEATH_PATTERNS[2] = {
  trigger   = "3+ enemies in adjacent tiles simultaneously",
  cause     = "Multiple melee attacks per turn",
  solution  = "Retreat to nearest corridor entrance. Fight enemies one at a time "
            .. "through the chokepoint. If no corridor, use a confusion or speed potion.",
  run_count = 0,
}

-- ── Pattern 003: Poison without antidote ──────────────────────────────────────
DEATH_PATTERNS[3] = {
  trigger   = "bitten by spider at low hp with no antidote",
  cause     = "Poison damage over time",
  solution  = "Always carry at least one antidote. If poisoned with no antidote, "
            .. "drink a healing potion immediately to buy time.",
  run_count = 0,
}

-- ── Pattern 004: Starved to death ─────────────────────────────────────────────
DEATH_PATTERNS[4] = {
  trigger   = "hunger reached 0 with no food in inventory",
  cause     = "Starvation",
  solution  = "Pick up ALL food items on every floor. Eat proactively when hunger < 40. "
            .. "Do not descend without food unless floor is cleared.",
  run_count = 0,
}

-- ── Pattern 005: Arrow trap ───────────────────────────────────────────────────
DEATH_PATTERNS[5] = {
  trigger   = "walked over suspicious floor tile while at low HP",
  cause     = "Arrow trap at critical HP",
  solution  = "At HP < 50%, move diagonally to probe for traps. "
            .. "If floor tile looks 'different', circle around it.",
  run_count = 0,
}

-- ── Pattern 006: Bear trap + swarm ───────────────────────────────────────────
DEATH_PATTERNS[6] = {
  trigger   = "stepped on bear trap with 3+ enemies nearby",
  cause     = "Bear trap root + enemy swarm",
  solution  = "Scan floor before moving into a room. Bear traps are VISIBLE — "
            .. "they appear as jaw icons on the floor.",
  run_count = 0,
}

-- ── Helper function for the bot to check if a situation matches a pattern ──────
function matches_death_pattern(situation_desc)
  for _, pattern in ipairs(DEATH_PATTERNS) do
    -- Simple substring match; the AI does richer semantic matching.
    if string.find(situation_desc, pattern.trigger, 1, true) then
      return pattern
    end
  end
  return nil
end

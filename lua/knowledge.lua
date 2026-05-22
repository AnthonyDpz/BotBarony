--[[
  knowledge.lua — Barony Game Mechanics Knowledge Base
  =====================================================
  This file is read by the AI bot at the start of each run to understand
  fundamental game mechanics. It is updated by the post-run analyst when
  new facts are discovered.

  Format: Lua comments + tables for structured data.
  The bot receives this file verbatim as part of its system prompt context.
--]]

-- ── Core Movement ─────────────────────────────────────────────────────────────

MOVEMENT = {
  -- Barony uses WASD for cardinal movement, QE/ZC for diagonals.
  keys = {
    north     = "w",
    south     = "s",
    east      = "d",
    west      = "a",
    northeast = "e",
    northwest = "q",
    southeast = "c",
    southwest = "z",
  },
  -- Movement is tile-based. Each key press moves exactly one tile.
  -- Holding a key does NOT sprint — press repeatedly.
  tile_based = true,
}

-- ── Combat ────────────────────────────────────────────────────────────────────

COMBAT = {
  -- Primary attack: Space bar (melee in the direction the player faces).
  primary_attack_key = "space",

  -- Ranged attack: hold Space + direction (for thrown weapons / bows).
  -- Not reliable without a ranged weapon equipped.

  -- Attacking costs no MP for melee.
  -- Magic attacks cost MP.

  tips = {
    "Always attack from the side or behind for bonus damage (sneak attack).",
    "Retreat to heal if HP falls below 30% — potions are precious.",
    "Ranged enemies (skeleton archers) should be closed to melee range fast.",
    "Berserking enemies deal double damage — disengage and kite.",
    "Doors can block enemy line-of-sight and melee — use them defensively.",
  },
}

-- ── Common Enemies ─────────────────────────────────────────────────────────────

ENEMIES = {
  skeleton = {
    hp         = "low",
    damage     = "low",
    speed      = "medium",
    weakness   = "blunt weapons (clubs, hammers) deal +50% damage",
    danger     = 2,   -- 1=trivial, 5=deadly
    notes      = "Undead — fire and holy spells deal bonus damage.",
  },
  rat = {
    hp         = "very low",
    damage     = "trivial",
    speed      = "fast",
    weakness   = "any weapon",
    danger     = 1,
    notes      = "Pack tactics — 3+ rats can overwhelm low-level characters.",
  },
  troll = {
    hp         = "high",
    damage     = "high",
    speed      = "slow",
    weakness   = "fire deals double damage; trolls regenerate HP",
    danger     = 4,
    notes      = "DO NOT fight trolls without fire. Kite and use fire potions.",
  },
  gnome = {
    hp         = "low",
    damage     = "medium",
    speed      = "fast",
    weakness   = "none notable",
    danger     = 2,
    notes      = "Thieves — can steal items. Kill fast or lock in a corridor.",
  },
  spider = {
    hp         = "medium",
    damage     = "medium",
    speed      = "medium",
    weakness   = "fire",
    danger     = 3,
    notes      = "Poison attack — apply antidote immediately after being bitten.",
  },
  minotaur = {
    hp         = "very high",
    damage     = "very high",
    speed      = "medium",
    weakness   = "none; avoid",
    danger     = 5,
    notes      = "Boss-tier. Do not engage without full HP and strong weapon.",
  },
}

-- ── Items ─────────────────────────────────────────────────────────────────────

ITEMS = {
  priority_pickup = {
    "potion of healing",
    "potion of extra healing",
    "food",
    "scroll of identify",
    "scroll of enchant weapon",
    "leather armor",
    "iron armor",
  },
  do_not_pickup = {
    -- Too heavy for utility gained in early game.
    "boulder",
    -- Cursed items cannot be removed without a remove curse scroll.
    -- Always identify before equipping.
  },
  potions = {
    healing         = "Restores 30-50 HP.",
    extra_healing   = "Restores 60-100 HP. Save for emergency.",
    speed           = "Doubles movement for 30 turns.",
    strength        = "Adds +4 STR for 50 turns.",
    invisibility    = "Enemies cannot target you for 60 turns.",
    confusion       = "CURSED — avoid drinking unless water-tested.",
    acid            = "Thrown at enemies: corrodes armor.",
  },
}

-- ── Traps ─────────────────────────────────────────────────────────────────────

TRAPS = {
  arrow_trap = {
    detection  = "tile looks slightly different; walk diagonally to avoid",
    danger     = 2,
    notes      = "Deals 5-15 piercing damage. Triggered by walking directly on tile.",
  },
  bear_trap = {
    detection  = "visible on floor as a metal jaw object",
    danger     = 3,
    notes      = "Roots the player for several turns. Enemies will swarm — panic.",
  },
  teleport_trap = {
    detection  = "subtle shimmer — step back when floor looks 'off'",
    danger     = 2,
    notes      = "Teleports player to a random location. Disorienting, not lethal.",
  },
  fire_trap = {
    detection  = "scorch marks around the tile",
    danger     = 4,
    notes      = "Ignites player. Roll to extinguish (move repeatedly). High damage.",
  },
}

-- ── Dungeon Structure ──────────────────────────────────────────────────────────

DUNGEON = {
  levels = {
    [1]  = { theme = "dungeon",  difficulty = "easy",    notes = "Tutorial zone. Collect gear." },
    [5]  = { theme = "swamp",    difficulty = "medium",  notes = "More traps. Spiders common." },
    [10] = { theme = "ruins",    difficulty = "hard",    notes = "Magic enemies. Identify wands." },
    [15] = { theme = "caves",    difficulty = "very hard", notes = "Minotaurs spawn regularly." },
    [25] = { theme = "boss",     difficulty = "extreme", notes = "Final boss area." },
  },
  stairs_key = ">",   -- Press > while standing on downstairs tile.
  -- Always descend when: good HP, decent weapon, food stocked.
  -- Never descend when: below 50% HP, no food, no weapon.
}

-- ── Character Classes ──────────────────────────────────────────────────────────

CLASSES = {
  human = {
    strengths = "Balanced stats; can use all equipment",
    weaknesses = "No special abilities",
    strategy   = "Collect strong weapon early. Prioritise armor.",
    recommended_for = "beginners",
  },
  skeleton = {
    strengths  = "No hunger; immune to poison; free undead racial",
    weaknesses = "Cannot use most food items; weak to holy damage",
    strategy   = "Ignore food items. Focus on weapons.",
    recommended_for = "intermediate",
  },
  vampire = {
    strengths  = "Heals on blood; strong melee",
    weaknesses = "Sunlight damage; needs blood to survive",
    strategy   = "Drain enemies frequently. Avoid lit areas.",
    recommended_for = "advanced",
  },
}

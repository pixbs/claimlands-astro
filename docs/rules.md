# Game rules

This is the source of truth for the requested game behavior. It records design requirements, not features already implemented. The current scaffold contains world/level/rendering primitives only. Numbered open decisions in [decisions.md](decisions.md) block their dependent gameplay work; agents must not silently settle them.

## World, factions and territory

The board is a procedural spherical grid with water and grass. Up to four factions use the fixed colors RED, YELLOW, GREEN and BLUE. Playable levels have two to four participants. A territory is a connected component of occupied land belonging to one faction; each territory holds its own wheat and gold at its capital.

Prices and resources are territory-local, not faction-wide. Turns belong to factions, so a faction can act in all of its territories during its turn.

## Tile economy

| Tile content | Per-turn effect while occupied | Construction/capture behavior |
| --- | --- | --- |
| Empty | +1 wheat | Base land state |
| Capital | +1 wheat, +1 gold | Enemy capture changes it to empty and awards 25% of capital money; resource scope/rounding need D2 |
| Town | +2 gold for 3 wheat when funded | Build for 2 gold +1 per existing town in this territory; enemy capture destroys it |
| Forest | No wheat or gold | Occupation clears it to empty; spreading described below |
| Field | +2 wheat | Build for 1 gold +1 per existing field in this territory; enemy capture destroys it |

Build towns and fields inside owned territory. Tile yields are content-specific; do not add the empty-tile yield to every improvement. Unit upkeep receives wheat before town production. Town production is all-or-nothing per town: three towns with 7 available wheat produce 4 gold, spend 6 wheat and leave 1 wheat. The timing of income, upkeep and production must be fixed in D1.

A forest has configurable probability N, initially 10%, to create forest on a neighboring empty tile. It cannot grow onto fields, towns or capitals. Which cells may be targeted, the attempt cadence, and whether N applies per forest or per neighbor remain D5.

## Capitals, splits and merges

Every retained territory must have a capital. If its capital is destroyed, select a replacement approximately near the component's center, with seeded randomness among candidates. Prefer an empty tile, otherwise replace a town, otherwise a field. If the component contains only forest and no eligible tile, clear its occupation. The center metric, selection and unit interactions remain D3.

If capture divides a territory, each component without a capital receives one under that rule. Split the original treasury proportionally by the surviving occupied tile counts. The supplied example: a 16-tile territory with 15 gold and 15 wheat loses one tile and splits into components of 10 and 5 tiles; their treasuries become 10/10 and 5/5. This example establishes wheat as well as gold redistribution. Allocation rounding and ordering relative to captured-capital loot remain D2.

When allied territories reconnect, combine their budgets and retain one capital; the removed capital becomes empty. The requested rule refers both to the capital closest to the captured tile and to removing the older capital, and leaves equal-age behavior unfinished. D3 must settle precedence and ties before implementation.

## Units

Each occupied tile can hold at most one pawn, warrior or knight. A unit purchase uses its territory's treasury. Creation order must be retained for starvation tie-breaking.

| Unit | Purchase or upgrade | Per-turn upkeep | Enemy targets it may capture |
| --- | --- | --- | --- |
| Pawn | Buy for 1 gold +1 per existing unit of any type in the territory | 1 wheat | Empty, forest, field; no occupied enemy pawn |
| Warrior | Upgrade pawn for 1 gold +1 per existing warrior or knight in the territory | 2 wheat | Pawn targets plus enemy town, capital and normal pawn; no warrior |
| Knight | Upgrade warrior for 2 gold +2 per existing knight in the territory | 2 wheat +1 gold | Enemy pawns of every type, including knights |

Purchases and upgrade counts refer to existing units before the action. Upgrading requires and consumes that unit's available move. All units are forbidden from moving onto an allied unit. Warrior-versus-knight interaction and knight capture permissions for tile improvements must be stated explicitly in D4.

A pawn may move once per turn. The requested distances are up to four tiles inside its territory and one adjacent tile into enemy/unoccupied land to capture it. Whether these are alternative moves or the four-tile route can be followed by a capture, and whether units can move through occupied friendly tiles, remain D4. Higher-tier movement inheritance must be confirmed there too.

When resources cannot cover upkeep, knights die before warriors and warriors before normal pawns. Among normal pawns, the oldest dies first. D1 must settle within-tier ties, gold-only knight shortages, and the exact selection of deaths when upkeep differs. Towns only consume the wheat left after unit resolution.

## Turns and presentation

- Factions alternate turns, rather than alternating territories. Each unit with an available move or upgrade displays a rotating star.
- A capital displays the same indicator when its territory can afford at least one legal pawn, town or field purchase. Affordability must include legal placement, not only the price.
- The current player may undo their actions until committing the turn. Undo restores gameplay state, resources, random state and statistics together; turn commitment closes the undo boundary.
- In future multiplayer, other human participants' cameras follow the active player's moves. Camera behavior is presentation and does not change simulation state.

## Levels and campaign

Levels use versioned RON with generator version, map frequency, seed, sparse tile overrides, two to four faction settings, AI properties and victory configuration. Seed zero produces a blank water base before explicit editor overrides. Other seeds produce generated terrain. A compact RON serialization is the one-string interchange form; readable RON is used for campaign authoring.

The current schema supports the four factions, terrain, empty/forest/field/town/capital content, AI hostility/intelligence in 0–100, and an `Elimination` configuration. These values do not yet execute game rules. Unit placement, starting treasury and campaign progression require later versioned additions.

The MVP editor is an internal tool that edits the same data the game loads. Public editing, sharing and custom-level discovery come later. AI difficulty increases through campaign settings; its behavioral meaning and decision budget need D6.

## Victory, statistics and replay

The intended victory choices are full elimination or a sufficiently decisive territorial/economic advantage. Elimination is the initial schema option; majority/advantage thresholds need D6. The victory screen records turns taken, enemy units killed, own units lost, all-time gold and additional approved statistics. D6 defines counting so undo and replay cannot inflate totals.

The simulation design must support deterministic replay from committed turns. A replay viewer is a later feature. Multiplayer, science trees and player-facing editor/sharing are outside the MVP.

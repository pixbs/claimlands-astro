"""Create the approved milestone backlog once; GitHub Issues remain authoritative."""
import json
import subprocess

REPO = "pixbs/claimlands-astro"


def api(path, body=None):
    args = ["gh", "api", f"repos/{REPO}/{path}"]
    if body is not None:
        args += ["--method", "POST", "--input", "-"]
    return json.loads(subprocess.check_output(args, input=json.dumps(body) if body else None, text=True))


def main():
    milestones = ["Foundation", "Visual fidelity", "Simulation", "Single-player MVP", "Later features"]
    existing = {m["title"]: m["number"] for m in api("milestones?state=all&per_page=100")}
    for title in milestones:
        if title not in existing:
            existing[title] = api("milestones", {"title": title})["number"]
    rows = [
        ("Visual fidelity", "Match terrain, coastlines and pixel textures", "visuals/renderer", "#1; owner-reviewed rendered reference capture", "Fixed seed/camera/time comparisons against the external prototype; grass, water and coasts preserve the reference appearance. Test both browser backends and memory bounds."),
        ("Visual fidelity", "Match procedural fields, forests and houses", "visuals", "terrain fidelity", "Recreate reference mesh proportions, placement and textures; maintain mesh invariants and review fixed close-up views. No reference runtime ships."),
        ("Visual fidelity", "Match atmosphere, transparent clouds and background stars", "renderer", "terrain fidelity", "Fixed animation times and camera views match reference atmosphere and see-through clouds. Validate transparency ordering and both rendering backends."),
        ("Simulation", "Resolve capital, economy and turn-order decisions", "gameplay contracts", "docs/rules.md D1–D6; owner decisions required", "Record exact rounding, loot basis, capital merge ties, turn settlement order, movement interpretation, starvation, and victory thresholds. Include concrete examples before dependent implementation."),
        ("Simulation", "Implement atomic commands, faction turns and snapshot undo", "gameplay", "#1; resolved turn-order contract", "Human and AI inputs share validated commands. Rejection leaves state unchanged. Undo restores complete state/RNG/stats until end turn; all faction territories act in one turn."),
        ("Simulation", "Implement connected territories and capital allocation", "gameplay territories", "atomic commands; resolved capital decisions", "Capture, split and merge are atomic. Preserve resource conservation and the 16-tile/15-resource split example. Test capital relocation, eligibility priorities and forest-only abandonment."),
        ("Simulation", "Implement territory treasuries and local construction", "gameplay economy", "territories; settlement-order decision", "Implement empty/capital/town/field yields and local escalating costs. Units receive wheat first. Three towns with seven wheat produce four gold and spend six wheat. Test affordability and conservation."),
        ("Simulation", "Implement pawn movement, upgrades and capture", "gameplay combat", "territories/economy; movement and starvation decisions", "Implement class-specific capture, movement range, no allied stacking, upgrade move consumption, local unit pricing and starvation priority. Add property sequences and rejection tests."),
        ("Simulation", "Implement deterministic forest spreading", "gameplay world events", "atomic commands; turn settlement order", "Default ten percent chance spreads only to eligible adjacent empty cells; occupation clears forests. Version randomness and test fixed seeds and forbidden destinations."),
        ("Visual fidelity", "Design units, capitals and rotating action indicators", "visuals/application", "unit and affordability command contracts", "New designs receive owner review because the prototype lacks these models. Stars accurately reflect available moves/upgrades or legal capital purchases."),
        ("Single-player MVP", "Implement configurable deterministic opponents", "AI", "complete simulation", "Hostility/intelligence parameters affect documented choices; every action goes through shared validation. Fixed scenarios test legal commands, reproducibility and increasing challenge."),
        ("Single-player MVP", "Build an internal RON campaign editor", "editor", "validated level contract and simulation", "Create empty or seeded worlds, apply tile overrides and faction/AI settings, export readable and compact RON. Round-trip edited fixtures and validate errors before world creation."),
        ("Single-player MVP", "Create seeded campaign levels and progression", "levels/application", "internal editor; AI", "Version campaign fixtures and progression; validate every level and reproducible opening state. Include a playable preview of campaign selection and entry."),
        ("Single-player MVP", "Implement victory results and match statistics", "gameplay/application", "owner-approved victory thresholds; simulation", "Count turns, kills, starvation deaths, earned currency and agreed statistics. Test elimination and approved advantage victory with deterministic result fixtures."),
        ("Single-player MVP", "Add versioned persistence and replay recording", "application persistence", "atomic commands/undo/statistics", "Save resolved initial board, schema/generator/rules versions and committed commands. Test exact restoration, corrupt input rejection and replay hash equality."),
        ("Later features", "Build the session replay viewer", "application replay", "versioned recording", "Replay the saved initial board and committed commands with deterministic checkpoints, seeking and camera focus. No invented outcomes for incompatible rules versions."),
        ("Later features", "Design and implement a science tree", "gameplay extensions", "stable simulation; approved science rules", "Define narrow extension contracts, then add tests that old campaign rules and saves retain their behavior. Scope and balancing need a separate design decision."),
        ("Later features", "Publish a player level editor and sharing format", "editor/sharing", "internal editor; versioned validation", "Specify user-facing editing, share limits and format migration. Treat shared levels as untrusted input and preserve deterministic resolution."),
        ("Later features", "Add multiplayer through the command protocol", "networking", "stable simulation/replay; approved network authority model", "Reuse command validation and deterministic state hashes; test ordering, duplicates, reconnection and version mismatch. Remote moves focus the observing camera."),
        ("Single-player MVP", "Verify physical mobile appearance and performance", "platform/renderer", "all visual and simulation MVP work", "Record target-device frame time, memory, thermal/lifecycle/input behavior and appearance. Set measured budgets and run the full runtime suites before release."),
    ]
    known = {i["title"] for i in api("issues?state=all&per_page=100")}
    for milestone, title, subsystem, prerequisites, acceptance in rows:
        if title in known:
            continue
        body = f"Owned subsystem: {subsystem}\n\nPrerequisites: {prerequisites}\n\nAcceptance: {acceptance}\n\nRead docs/rules.md and relevant contracts. Required automated gates and an actual-game preview apply. Unresolved owner decisions block their behavior; do not invent mechanics. Keep each implementation PR bounded and use linked follow-up issues when this design needs subdivision."
        issue = api("issues", {"title": title, "body": body, "milestone": existing[milestone]})
        print(issue["html_url"], title)


if __name__ == "__main__":
    main()

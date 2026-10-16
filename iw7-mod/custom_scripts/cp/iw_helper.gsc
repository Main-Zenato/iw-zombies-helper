// iw-helper extras for Zombies, run by iw7-mod's script loader.
//
// iw7-mod only: the Steam game never loads custom_scripts, and this file must stay that way. It calls the
// game's own Zombies functions, which the external tool cannot reach from memory (Pack-a-Punch, bonuses).
//
// Install: copy the iw7-mod folder of this repository into the game folder, so that this file ends up in
// <game folder>\iw7-mod\custom_scripts\cp\iw_helper.gsc. It is compiled when a Zombies match loads.
//
// Two ways to send a command, as the host:
// - the iw-helper window, page Match, section "iw7-mod extras": it writes the command's number into
//   level.command_given (the mailbox below);
// - the iw7-mod console: `set iwh <command>`, or a key bound once, for example: bind F5 "set iwh pap"
// Console commands:
//   pap            upgrade every player's weapons one Pack-a-Punch level (again for the next level)
//   killall        kill every zombie alive (bosses may be immune)
//   doors          turn on the power and open the doors
//   perks          give you every perk of the map
//   weapon         give you a random weapon from the map's list
//   drop <bonus>   drop a bonus in front of you: ammo, kill, instakill, cash, upgrade, fire, infinite,
//                  grenade, power, board (what exists depends on the map)
//   bonuses        list the bonuses of this map
//   help           list the commands

// The mailbox: iw-helper writes 7700 + a command number, the script runs it and writes
// 7700 back. Numbers kept in step with OneShot::script_code in src/trainer.rs.
init()
{
    setdvarifuninitialized("iwh", "");
    level.command_given = 7700;
    level thread watch_commands();
}

watch_commands()
{
    level endon("game_ended");

    for (;;)
    {
        wait 0.1;

        // The host: the console and the window are theirs.
        player = level.players[0];
        if (!isdefined(player))
            continue;

        if (level.command_given != 7700)
        {
            code = level.command_given - 7700;
            level.command_given = 7700;
            run_code(player, code);
        }

        line = getdvar("iwh");
        if (line == "")
            continue;
        setdvar("iwh", "");

        words = strtok(line, " ");
        argument = "";
        if (words.size > 1)
            argument = words[1];
        run_word(player, words[0], argument);
    }
}

// Commands from iw-helper, by number.
run_code(player, code)
{
    switch (code)
    {
        case 1: upgrade_weapons(player); break;
        case 2: kill_all(player); break;
        case 3: open_doors(player); break;
        case 4: give_perks(player); break;
        case 5: give_weapon(player); break;
        case 10: drop_bonus(player, "ammo"); break;
        case 11: drop_bonus(player, "kill"); break;
        case 12: drop_bonus(player, "instakill"); break;
        case 13: drop_bonus(player, "cash"); break;
        case 14: drop_bonus(player, "fire"); break;
        case 15: drop_bonus(player, "infinite"); break;
        default: player iprintln("iw-helper: unknown command " + code); break;
    }
}

// Commands from the console, by name.
run_word(player, command, argument)
{
    switch (command)
    {
        case "pap": upgrade_weapons(player); break;
        case "killall": kill_all(player); break;
        case "doors": open_doors(player); break;
        case "perks": give_perks(player); break;
        case "weapon": give_weapon(player); break;
        case "drop": drop_bonus(player, argument); break;
        case "bonuses": list_bonuses(player); break;
        default: player iprintln("iw-helper: pap, killall, doors, perks, weapon, drop <bonus>, bonuses"); break;
    }
}

// The same function as the "upgrade" bonus and the boss rewards: every weapon that can go one level
// higher does, and the weapon in hand is switched to its new version.
upgrade_weapons(player)
{
    if (!isdefined(level.upgrade_weapons_func))
    {
        player iprintln("iw-helper: no Pack-a-Punch on this map");
        return;
    }

    level thread [[level.upgrade_weapons_func]]();
    player iprintln("iw-helper: weapons upgraded");
}

kill_all(player)
{
    enemies = scripts\cp\cp_agent_utils::get_alive_enemies();
    count = 0;

    foreach (enemy in enemies)
    {
        if (!isdefined(enemy) || !isalive(enemy))
            continue;

        enemy dodamage(enemy.health + 100, enemy.origin);
        count++;
    }

    player iprintln("iw-helper: " + count + " zombies killed");
}

// What the game's direct boss fight mode runs at the start: each map's own way to turn on the power
// and open its doors (or the generic one: generators, paid doors, chi doors, team doors). Calling it a
// second time does nothing.
open_doors(player)
{
    level thread scripts\cp\zombies\direct_boss_fight::open_sesame();
    player iprintln("iw-helper: power on, doors opening");
}

// Every perk the map registered, given the way a perk machine does.
give_perks(player)
{
    if (!isdefined(level.coop_perk_callbacks))
    {
        player iprintln("iw-helper: no perks on this map");
        return;
    }

    count = 0;
    foreach (perk in getarraykeys(level.coop_perk_callbacks))
    {
        if (player scripts\cp\utility::has_zombie_perk(perk))
            continue;

        player scripts\cp\zombies\zombies_perk_machines::give_zombies_perk(perk, 0);
        count++;
        wait 0.1;
    }

    player iprintln("iw-helper: " + count + " perks given");
}

// A random weapon from the map's list (wall weapons and the mystery box's), at level 1. If the player
// already carries as many weapons as allowed, the one in hand is taken, as the box does.
give_weapon(player)
{
    if (!isdefined(level.magic_weapons) || level.magic_weapons.size == 0)
    {
        player iprintln("iw-helper: no weapon list on this map");
        return;
    }

    names = getarraykeys(level.magic_weapons);
    weapon = level.magic_weapons[names[randomint(names.size)]];

    if (scripts\cp\zombies\zombies_weapons::should_take_players_current_weapon(player))
    {
        current = player getcurrentweapon();
        base = scripts\cp\utility::getrawbaseweaponname(current);
        player takeweapon(current);
        if (isdefined(player.pap[base]))
            player.pap[base] = undefined;
    }

    given = player scripts\cp\utility::_giveweapon(weapon, undefined, undefined, 0);
    if (!isdefined(given))
        given = weapon;

    // The level Pack-a-Punch reads to upgrade it later.
    entry = spawnstruct();
    entry.lvl = 1;
    player.pap[scripts\cp\utility::getrawbaseweaponname(given)] = entry;
    player notify("weapon_level_changed");
    player switchtoweapon(given);
    player iprintln("iw-helper: weapon given (" + given + ")");
}

// Bonuses are named after the map's loot table, such as "kill_50" or "ammo_max": the first one whose
// name is the argument, or starts with it, is dropped.
drop_bonus(player, kind)
{
    if (!isdefined(level.loot_id))
    {
        player iprintln("iw-helper: no bonuses on this map");
        return;
    }

    if (kind == "")
    {
        player iprintln("iw-helper: drop what? (set iwh bonuses)");
        return;
    }

    foreach (id in getarraykeys(level.loot_id))
    {
        parts = strtok(id, "_");
        if (id != kind && parts[0] != kind)
            continue;

        spot = player.origin + anglestoforward(player.angles) * 80;
        scripts\cp\loot::drop_loot(spot, undefined, id);
        player iprintln("iw-helper: bonus dropped (" + id + ")");
        return;
    }

    player iprintln("iw-helper: no " + kind + " bonus on this map");
}

list_bonuses(player)
{
    if (!isdefined(level.loot_id))
    {
        player iprintln("iw-helper: no bonuses on this map");
        return;
    }

    names = "";
    foreach (id in getarraykeys(level.loot_id))
        names = names + id + " ";

    player iprintln("iw-helper bonuses: " + names);
}

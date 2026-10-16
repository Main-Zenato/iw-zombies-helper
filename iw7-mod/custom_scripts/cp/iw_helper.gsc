// iw-helper extras for Zombies, run by iw7-mod's script loader.
//
// iw7-mod only: the Steam game never loads custom_scripts, and this file must stay that way. It calls the
// game's own Zombies functions, which the external tool cannot reach from memory (Pack-a-Punch, bonuses).
//
// Install: copy the iw7-mod folder of this repository into the game folder, so that this file ends up in
// <game folder>\iw7-mod\custom_scripts\cp\iw_helper.gsc. It is compiled when a Zombies match loads.
//
// Use, as the host: open the iw7-mod console and type `set iwh <command>`, or bind a key once:
//   bind F5 "set iwh pap"
// Commands:
//   pap            upgrade every player's weapons one Pack-a-Punch level (again for the next level)
//   killall        kill every zombie alive (bosses may be immune)
//   drop <bonus>   drop a bonus in front of you: ammo, kill, instakill, cash, upgrade, fire, infinite,
//                  grenade, power, board (what exists depends on the map)
//   bonuses        list the bonuses of this map
//   help           list the commands

init()
{
    setdvarifuninitialized("iwh", "");
    level thread watch_commands();
}

watch_commands()
{
    level endon("game_ended");

    for (;;)
    {
        wait 0.25;

        line = getdvar("iwh");
        if (line == "")
            continue;
        setdvar("iwh", "");

        // The host: the console that set the variable is theirs.
        player = level.players[0];
        if (!isdefined(player))
            continue;

        words = strtok(line, " ");
        command = words[0];
        argument = "";
        if (words.size > 1)
            argument = words[1];

        switch (command)
        {
            case "pap":
                upgrade_weapons(player);
                break;
            case "killall":
                kill_all(player);
                break;
            case "drop":
                drop_bonus(player, argument);
                break;
            case "bonuses":
                list_bonuses(player);
                break;
            default:
                player iprintln("iw-helper: pap, killall, drop <bonus>, bonuses");
                break;
        }
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

    player iprintln("iw-helper: unknown bonus " + kind + " (set iwh bonuses)");
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

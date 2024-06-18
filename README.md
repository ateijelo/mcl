# mcl
Command line tools for Minecraft

# reset-lighting

Run `mcl reset-lighting path/to/region-file/r.3.-1.mca`. This will delete the `isLigthOn`, `BlockLight` and `SkyLight` entries from all chunks, reseting the chunk lighting info.

# prune

This is meant to prune chunks based on InhabitedTime as an alternative to the equivalent functionality in mcaselect.

# blocks

Search and print the positions of specific blocks. I used this to compare diamond distribution between 1.20.1 and 23w31a.

# block-entities

Show block entities. This is useful to inspect chests, barrels, shulker boxes, and almost anything with an inventory in it.
For instance, a command like this:

```bash
mcl block-entities \
    --world /home/user/.minecraft/saves/New\ World \
    --dimension overworld \
    --from 1482,172,396 --to 1482,172,395 \
    --json
```

would be useful to inspect a double chest at the specified coordinates. The output would look something like this:

```json
{"y":172,"keepPacked":0,"Items":[{"Slot":0,"count":5,"id":"minecraft:firework_rocket"},{"count":64,"Slot":1,"id":"minecraft:firework_rocket"},{"Slot":2,"id":"minecraft:firework_rocket","count":64},{"count":64,"Slot":3,"id":"minecraft:firework_rocket"},{"count":64,"Slot":4,"id":"minecraft:firework_rocket"},{"Slot":5,"count":64,"id":"minecraft:firework_rocket"},{"Slot":6,"id":"minecraft:firework_rocket","count":64},{"id":"minecraft:firework_rocket","Slot":7,"count":64},{"count":64,"Slot":8,"id":"minecraft:firework_rocket"},{"Slot":9,"id":"minecraft:firework_rocket","count":64},{"id":"minecraft:firework_rocket","Slot":10,"count":64},{"Slot":11,"count":64,"id":"minecraft:firework_rocket"},{"count":41,"Slot":12,"id":"minecraft:firework_rocket"}],"z":396,"x":1482,"id":"minecraft:chest"}
{"id":"minecraft:chest","Items":[],"keepPacked":0,"y":172,"x":1482,"z":395}
```

Since Minecraft saves chunks quite frequently, you can inspect running farms by repeating that command. Combine that with `jq` for some filtering and you get a powerful tool.

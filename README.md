# Papers Tools

## Unpacking `Art.dat`:

```bash
./papers-tools -i ./Art.dat -o ./out -g /path/to/game/dir
```

## Packing a folder into the `Art.dat` format:

```bash
./papers-tools -i ./dir -o Art-modded.dat -g /path/to/game/dir
```

The first folder will not be included in the asset names when packing. So make sure your folder structure is something like:
* `./dir/assets/some_texture.png`
* `./dir/assets/some_other_texture.png`
* `./dir/assets/fonts/some_font.fnt`

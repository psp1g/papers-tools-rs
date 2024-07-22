# Papers Tools

## Unpacking `sharedassets0.assets`:

```bash
./papers-tools -g /path/to/steamapps/common/PapersPlease unpack
```

You can also pass the Art.dat file as the input directly.

## Patching 

To overwrite textures, just add the new texture in the same path as the original one in your patch directory. For example,
to replace the `RifleTranq.png` texture, put the new texture in `patch/assets/textures/RifleTranq.png`.  

Editing xml files is also supported. The tool will merge your changes with the original game xml files. Example for adding a new
paper (`patch/assets/data/Papers.xml`):

```xml
<?xml version="1.0" encoding="UTF-8" ?>
<papers>
    <paper id="ExamplePaper" outer="ExamplePaperOuter.png" reveal="fromslot" oddshape="true">
        <page image="ExamplePaperInner.png"/>
    </paper>
</papers>
```

To override entire sections that don't have an id or attributes, add `id="override"` to the base element:

```xml
<?xml version="1.0" encoding="UTF-8" ?>
<facts>
    <nations id="override">
        <nation name="Thalvoria" cities="Ebonvale;Aerofell;Silvanwood" citizen="Thalvorian"/>
        <nation name="Vectis" cities="Nexaria;Azurea;Agrona" citizen="Vectisan"/>
    </nations>
</facts>
```

for more information on the format, just check out the unpacked files.  

You can even replace audio files, but a separate json file is needed to map the original audio asset object to the new one.  
Example (json path: `patch/audio_patches.json` audio path: `patch/audio/awp.fsb`):

```json
[
  {
    "objectName": "border-gunshot",
    "patchedPath": "audio/awp.fsb",
    "loadType": 0,
    "channels": 2,
    "frequency": 48000,
    "bitsPerSample": 16,
    "length": 4.642667,
    "isTrackerFormat": false,
    "subsoundIndex": 0,
    "preloadAudioData": false,
    "loadInBackground": false,
    "legacy3d": true,
    "compressionFormat": "adpcm"
  }
]
```

**The audio files need to be unity compatible fmod sound banks. Any other format will not work.**

To apply the patch, run:

```bash
./papers-tools -g /path/to/steamapps/common/PapersPlease patch -p /path/to/patch
```

`-p /path/to/patch` can be omitted if the patch is in the default path `./patch`.

## Reverting

To revert the changes made by the patch, run:

```bash
./papers-tools -g /path/to/steamapps/common/PapersPlease revert
```

For less common used commands, check the built-in help:

```bash
./papers-tools -h
```

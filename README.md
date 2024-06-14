# Papers Tools

## Unpacking `sharedassets0.assets`:

```bash
./papers-tools -g /path/to/steamapps/common/PapersPlease unpack
```

You can also pass the Art.dat file as the input directly.

## Patching 

To overwrite textures, just put the new texture in the same path as the original one, with the same name. For example,
to replace the `RifleTranq.png` texture, put the new texture in `patch/assets/textures/RifleTranq.png`.  
Editing xml files is also supported. The tool will merge your changes with the original file. Example for adding a new
paper (`patch/assets/data/Papers.xml`):

```xml
<?xml version="1.0" encoding="UTF-8" ?>
<papers>
    <paper id="ExamplePaper" outer="ExamplePaperOuter.png" reveal="fromslot" oddshape="true">
        <page image="ExamplePaperInner.png"/>
    </paper>
</papers>
```

for more information on the format, just check out the unpacked files.  
Replacing audio files is also supported, but a separate json file is needed to map the original audio file to the new 
one. Example (json path: `patch/audio_patches.json` audio path: `patch/audio/awp.fsb`):

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

Please note that the audio files need to be unity compatible fmod sound banks. Any other format will not work.

To apply the patch, run:

```bash
./papers-tools -g /path/to/steamapps/common/PapersPlease patch -p patch
```

`-p patch` can be omitted if the patch is in the default path `patch`.

## Reverting

To revert the changes made by the patch, run:

```bash
./papers-tools -g /path/to/steamapps/common/PapersPlease revert
```

For less common used commands, check the built-in help:

```bash
./papers-tools -h
```
"Assets/Resources" is a special folder that code can access.
If you change a folder name, the code needs to reflect this change.

Anyway, this folder is primarily used for generation and image tilemaps.
If the genTileMap doesn't look clean, check the texture settings in the Inspector.
You'll want to set the filter mode to Point, and set compression to None.
sRGB sometimes messes with brightness depending on whether it's on or off.
Lastly, enable Read/Write so the code can actually access the pixel info.
## **M**ovieChapters**PLS**

##### no more chapter-less media!

___


**Usage:**
```
moviechapterspls 00000.mpls
ffmpeg -i 00000.m2ts -i 00000.m2ts.ff -map 0:v -map_metadata 1 -c copy 00000.mkv
```

___

This script reads the binary data from your `.mpls` file and creates `FFmetadata` chapter files for every media file (e.g. `00000.m2ts`), that has at least one chapter assigned.

* Chapter names are numbered like so: `Chapter 1`.

* A few test have shown that the calculations, which are done by the script, indeed frame-precise are.

___

**For the confused individuals:**

Importing `.mpls` chapters and splitting them for multiple files is a very manual task, something that this script tries to automate. It also helps a lot determining to which file chapters are supposed to go to.

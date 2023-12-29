## **M**ovieChapters**PLS**

##### no more chapter-less media!

___


**Usage:**
```
moviechapterspls 00000.mpls
mkvmerge "00000.m2ts" --chapters "00000.m2ts.xml" -o "00000.mkv"
```

```
moviechapterspls -F 00000.mpls
ffmpeg -i 00000.m2ts -i 00000.m2ts.ff -map 0:v -map_metadata 1 -c copy 00000.mkv
```

+ `-X` for XML chapters
+ `-F` for FFmetadata chapters

+ `--merge` to specify a range of m2ts ID's that will be used to create a merged chapter file
  + `-m 0 2` -> chapters from 0000**0**.m2ts, 00001.m2ts and 0000**2**.m2ts appended to one XML file.
  + Why: *Some Blu-rays split episodes into multiple files.*
  + This will only work if your last m2ts has at least one chapter.

+ `--only` to get *only* chapters from a given range, normalized to a starting position of 0.
  + `-o 01:05:48.485 02:03:00.4561` -> chapters from the first to the second timestamp appended to the XML (starting from time 0)
  + Why: *Some Blu-rays/DVDs have truncated episodes in one file.*
  + This will only work if the MPLS file has one file ID with the specified chapters (aka.: incompatible with `--merge`)
  + Note: Maybe run `moviechapterspls` first, to extract the correct timestamps.

+ `--output-dir` to specify the output folder

___

This script reads the binary data from your `.mpls` file and creates chapter files for every media file (e.g. `00000.m2ts`), that has at least one chapter assigned.

* Chapter names are numbered like so: `Chapter 1`.

* Calculations done by the script are frame-precise (at least with 24000/1001 fps).

___

**For the confused individuals:**

Importing `.mpls` chapters and splitting them for multiple files is a very manual task, something that this script tries to automate. It also helps a lot determining to which file, chapters are supposed to go to.

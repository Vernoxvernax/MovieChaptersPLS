## **M**ovieChapters**PLS**

Convert `.mpls` chapters using mkvmerge to xml. (`.mpls` aren't open-source, so nobody really knows how to read them, well, except for mkvmerge of course)
Well, mkvmerge is open-source, so maybe I'll just go read some of that in the future.

Then run:
```
chapterpls run -x chapters.xml -f Bluray/BDMV/STREAM/00001.m2ts ...
```

This script uses `ffmpeg` to determine the duration of each file, so make sure it's installed. That however also means that the output may sometimes include some of `ffmpeg`'s useless warnings.

So be sure to list all the files, if you want to create chapters or not doesn't matter. This script basically just stretches the chapters onto these, so if there are gaps, the chapters will not align anymore.
I also don't really know how standardized these chapters are, but those which I've encountered building this application, for example, have one chapter at exactly the end of a video file, which is why this is even working, so I'm sorry if your Blu-ray is not supported.

___

**For the confused individuals:**

Importing `.mpls` chapters and splitting them for multiple files is a very manual task, something that this script tries to automate. In the future I might try to read the `.mpls` files natively, which apparently also include video file information, so you wouldn't even need to specify video-input files anymore.

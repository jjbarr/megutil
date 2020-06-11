# Megutil
megutil is a small cross-plaform tool for extracting files from the .MEG
archives used by Petroglyph for their RTSes. It is extremely limited (only
supporting listing and extraction of the latest unencrypted format) and honestly
not the greatest tool for the job. However, I wanted to write something, and
this one runs on Linux.

Its primary use is probably extracting Frank Klepacki's music from Petroglyph
games so you can listen to it through VLC or something. That's why I wrote it,
anyways...

# Building
Just build it with cargo as standard.

# Usage
`megutil`'s option set is based on `tar`. If you're used to `tar`, you already
know most of the commands. If not, `megutil --help` will fill you in.


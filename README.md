# qable
Queue Downloads for Deluge

A Rust application to combine IMBD, Deluge and Plex to find and manage media.


TODO
-----
TODO: split into individual tools joined by a single application

TODO: check for currently downloading and queued by qable
Create database (file) of Pending, Downloading, Downloaded, Existing
search existing plex library

TODO: -v verbose mode, -l log file location

TODO: -l can accept multiple lists

TODO: -c clean plex library
use database to cache already synced imdb_ids, names, posters, etc...
Delete duplicate movies from library (keep one which most closely matches size)
Find unmatched movies (have no plex guid)
 Example:
 "guid": "local://1881",
 "type": "movie",
 "title": "Fyre the Greatest Party That Never Happened",

TODO: -o optimize plex library
Replace movies with files of desired file size (if file is SIZE times larger than target replace it if a better match exists)
Replace movies with files of desired encoding, or resolution

TODO: -a analyze plex library
size, length, bitrate, resolution, 

TODO: add minimum imdb rating to config
TODO: add restart option that will pick up list download from last spot



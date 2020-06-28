# qable
Queue Downloads for Deluge


TODO: split into individual tools joined by a single application
TODO: check for currently downloading and queued by qable
Create database (file) of Pending, Downloading, Downloaded, Existing
search existing plex library
TODO: -v verbose mode, -l log file location
TODO: -l can accept multiple lists
TODO: -e export imdb_id list from plex server
TODO: -c clean names in plex library
Delete duplicate movies from library
Replace movies with files of desired file size (if file is SIZE times larger than target replace it if a better match exists)
Replace movies with files of desired encoding, or resolution
Check files for errors that prevent playback and replace 
Find unmatched movies (have no plex guid)
 Example:
 "guid": "local://1881",
 "type": "movie",
 "title": "Fyre the Greatest Party That Never Happened",
TODO: add minimum imdb rating to config
TODO: add restart option that will pick up list download from last spot

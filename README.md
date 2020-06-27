# qable
Queue Downloads for Deluge


TODO: split into individual tools joined by a single application
TODO: check for currently downloading and queued by qable
Create database (file) of Pending, Downloading, Downloaded, Existing
search existing plex library
TODO: -v verbose mode, -l log file location
TODO: -l can accept multiple lists
TODO: -d delete duplicate movies from library
TODO: -c clean names in plex library
 from plex list "ratingKey": "1641", and guid imdb key (see existing plex list)
 then put_plex_movie_metadata
Find unmatched movies (have no plex guid)
 Example:
 "guid": "local://1881",
 "type": "movie",
 "title": "Fyre the Greatest Party That Never Happened",
TODO: add minimum imdb rating to config

TODO: add ability to compare plex display name with imdb name and fix
TODO: add restart option that will pick up list download from last spot

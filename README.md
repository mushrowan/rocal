# rocal: Ro's Calendar

## A CalDAV-compatible TUI calendar & planner application

## WORK IN PROGRESS

ro's day planner thing! because i'm disorganised and i need help

i'll maybe make a proper readme once this is ready for people to like, use it and stuff

yeah i'm kinda bad at rust, i started learning it a month ago at time of writing.
i'm happy with what i've done though!

## Todo

- I think that actually there just aren't many good cli caldav implementations
  at the moment. there's khal, but synchronisation for that requires vdirsyncer or
  some other helper to do it for you. And I think that makes it less accessible,
  really. From my own experience of trying to set it up, vdirsyncer is a bit janky
  and mandates stuff like connection pairs which just don't really fit into my
  workflow (and khal doesn't even have a notion of a collection anyway). So
  basically I've kinda decided that this project is actually going to become a
  full TUI calendar application. It seems like the Rust libdav crate has done a
  good job at implementing a way to interact with the caldav protocol itself, so
  now I just need to put all the blocks together.

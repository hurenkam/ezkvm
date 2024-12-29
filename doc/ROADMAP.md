# Roadmap

## before 0.1.0 release

- ~~Support for vnc protocol~~
- ~~Support 440fx~~
- ~~Support sata hd & cd/dvd~~
- ~~installer for debian bookworm~~
- ~~installer for ubuntu noble numbat (24.04 LTS)~~
- ~~installer for arch linux~~

## short term

- reduce overhead a.o. in storage subsystem
- fix resources
- Run macos using ezkvm (and create an example config file for it)

## long term

- automate tests for examples
- implement templates
- Improve support for network and storage devices
- Create rpm package

## things to investigate

- Split storage and network items into netdev+device and drive+device items
- A templating system could make things easier and more generic, where type indicates a template
  rather than a type, and set defaults for netdev/drive/device sections rather than implement
  complete structs for them.

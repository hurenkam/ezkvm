#!/usr/bin/bash

wget https://raw.githubusercontent.com/hurenkam/ezkvm/refs/heads/stable/arch/PKGBUILD
wget https://raw.githubusercontent.com/hurenkam/ezkvm/refs/heads/stable/arch/proto.install
makepkg --skipinteg


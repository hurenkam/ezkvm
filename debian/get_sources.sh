#!/usr/bin/bash

NAME=ezkvm
VERSION=0.1.0
SOURCE=stable.tar.gz
SOURCE_DIR=ezkvm-stable
TARGET_DIR=${NAME}-${VERSION}
TARGET_ZIP=${NAME}_${VERSION}.orig.tar.gz

wget https://github.com/hurenkam/ezkvm/archive/refs/heads/${SOURCE}
mkdir $TARGET_DIR && tar xvzf $SOURCE -C $TARGET_DIR --strip-components 1
tar cvzf $TARGET_ZIP $TARGET_DIR

rm $SOURCE


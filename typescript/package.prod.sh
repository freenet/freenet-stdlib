#!/usr/bin/bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
PKG_DIR=$(mktemp -d)
./package.dev.sh "$PKG_DIR"
echo "Packages at $PKG_DIR"
OUTPUT=$(find $PKG_DIR -type f -name '*.tgz')
echo "Publishing $OUTPUT ..."
read -p "Proceed publishing? " -n 1 -r
echo   
if [[ $REPLY =~ ^[Yy]$ ]]
then
	npm publish $OUTPUT
else 
	echo "Not publishing"
fi

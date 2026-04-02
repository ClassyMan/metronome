#!/bin/bash
ninja -C ~/gnome-metronome/builddir && \
  sudo cp ~/gnome-metronome/builddir/data/resources/resources.gresource /usr/local/share/metronome/resources.gresource && \
  sudo cp ~/gnome-metronome/builddir/src/release/metronome /usr/local/bin/metronome && \
  echo "Installed successfully"

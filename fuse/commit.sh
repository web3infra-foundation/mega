#! /bin/bash
pid=$(pgrep fuse)
kill -10 $pid
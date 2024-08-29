#!/bin/bash

# user must set the MEGA_HOST,MEGA_INTERNAL_HOST and MOON_HOST
if [ -z "$MEGA_HOST" ]; then 
  echo "MEGA_HOST is not set"
  exit 1
fi

if [ -z "$MEGA_INTERNAL_HOST" ]; then 
  echo "MEGA_INTERNAL_HOST is not set"
  exit 1
fi

if [ -z "$MOON_HOST" ]; then 
  echo "MOON_HOST is not set"
  exit 1
fi

exec node server.js
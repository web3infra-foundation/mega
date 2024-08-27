#!/bin/bash

# user must set the NEXT_PUBLIC_API_URL
if [ -z "$MEGA_HOST" ]; then 
  echo "MEGA_HOST is not set"
  exit 1
fi

if [ -z "$MOON_HOST" ]; then 
  echo "MOON_HOST is not set"
  exit 1
fi

exec node server.js
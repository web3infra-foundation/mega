#!/bin/bash
NEXT_PUBLIC_CALLBACK_URL=http://0.0.0.0:3000/auth/github/callback

# user must set the NEXT_PUBLIC_API_URL
if [ -z "$NEXT_PUBLIC_API_URL" ]; then 
  echo "NEXT_PUBLIC_API_URL is not set"
  exit 1
fi

# write the environment variables to a file
echo "NEXT_PUBLIC_API_URL=$NEXT_PUBLIC_API_URL" > .env.local
echo "NEXT_PUBLIC_CALLBACK_URL=$NEXT_PUBLIC_CALLBACK_URL" >> .env.local

# TODO: run `npm run s start` didn't work, use `npm run dev` temporarily
exec npm run dev
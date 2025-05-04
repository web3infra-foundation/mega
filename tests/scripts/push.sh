#!/bin/bash

# Specify the directory to check
parent_directory="$1"

# Check if the directory exists
if [ ! -d "$parent_directory" ]; then
  echo "The specified directory does not exist."
  exit 1
fi

# Initialize a counter for numbering
counter=1

# Traverse the first-level subdirectories of parent_directory
for subdir in "$parent_directory"/*; do
  # Check if $subdir is empty after processing
  if [ -z "$(ls -A "$subdir")" ]; then
   echo "$subdir is empty, rm $subdir"
   rmdir "$subdir"
  fi

  # Check if it's a directory
  if [ -d "$subdir" ]; then
    echo "==========="
    echo "Processing directory: $subdir"

    # Traverse the first-level subdirectories of subdir
    for nested_dir in "$subdir"/*; do

      echo "-------------"
      echo "Task Number: $counter - $nested_dir"

      # Enter the nested directory
      cd "$nested_dir" || exit 1

      # Get the
      REPONAME=$(basename `git rev-parse --show-toplevel`)
      NAMESPACE=$(basename "$subdir")

      git remote remove mega
      git remote add mega "http://127.0.0.1:8000/third-party/rust/$NAMESPACE/$REPONAME"

      git push --all mega

      # Increment the counter
      counter=$((counter + 1))
    done
  fi
done

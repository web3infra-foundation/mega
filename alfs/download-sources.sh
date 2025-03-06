#!/bin/sh

if [[ $# -ne 2 ]]; then
    echo "Usage: $0 <file_list> <target_directory>"
    exit 1
fi

file_list="$1"
target_dir="$2"
download_host="https://file.gitmega.net/lfs"

if [[ ! -f "$file_list" ]]; then
    echo "Error: File list '$file_list' does not exist."
    exit 1
fi

if [[ ! -d "$target_dir" ]]; then
    echo "warning: Target directory '$target_dir' does not exist. Creating it."
    mkdir -pv "$target_dir"
fi

while IFS= read -r file; do
    [[ -z "$file" ]] && continue
    url="$download_host/$file"
    target_path="$target_dir/$file"

    echo "Downloading: $url -> $target_path"

    curl -L --fail --retry 3 --retry-delay 5 -o "$target_path" "$url"

    if [[ $? -ne 0 ]]; then
        echo "Error downloading $file"
    else
        echo "Successfully downloaded $file"
    fi
done <"$file_list"

echo "All downloads complete."

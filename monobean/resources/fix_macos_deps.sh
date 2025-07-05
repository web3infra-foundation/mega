#!/bin/sh

version=1.6

usage() {
    # Clean and simple help without problematic color codes
    cat << 'EOF'
Usage: binary_dylib_tool [OPTION] BINARY_FILE
Bundle third party dylibs for BINARY_FILE and fix up linkages.

The dylibs are copied into the specified output directory or ./patched_dylibs by default.

  -h, --help, --usage                Show this help screen and exit.
  -v, --version                      Show version information and exit.
  -l, --list                         Only list dylibs used by binary, do not copy or link.
  -o, --output DIR                   Output directory for dylibs (default: ./patched_dylibs).
  -r, --refresh                      Clean output cache and run cargo clean before processing.
  --rpath PATH                       Manually specify rpath for the binary (e.g., @executable_path/../Frameworks).

Examples:
  binary_dylib_tool ./mybinary                              # bundle and link ./mybinary
  binary_dylib_tool -o /path/to/libs ./mybinary             # use custom output directory
  binary_dylib_tool --list ./mybinary                       # list third party libs used by ./mybinary
  binary_dylib_tool --refresh ./mybinary                    # clean caches and process ./mybinary
  binary_dylib_tool --rpath @executable_path/../Frameworks ./mybinary  # use custom rpath

Original project homepage and documentation: <https://github.com/rkitover/mac-third-party-libs-tool>
Customized by Monobean for better compatibility with GTK-rs applications and packaging tools.
EOF
}

# Check if a binary file is valid and can be processed
check_binary_integrity() {
    local file="$1"
    
    # Check if file exists and is readable
    [ -f "$file" ] || return 1
    
    # Try to run otool -L on the file to check if it's valid for our purposes
    # We only need to check if we can read the load commands, not the full structure
    if ! otool -L "$file" >/dev/null 2>&1; then
        return 1
    fi
    
    return 0
}

# Safely remove code signature from a binary file
safe_remove_signature() {
    local file="$1"
    
    # Check if file has a signature first
    if codesign -d "$file" 2>/dev/null; then
        # File is signed, remove signature
        codesign --remove-signature "$file" 2>/dev/null || true
    fi
    
    return 0
}

# Safely add ad-hoc signature to a binary file
safe_add_signature() {
    local file="$1"
    
    # Add ad-hoc signature
    codesign -s - "$file" 2>/dev/null || true
    
    return 0
}

main() {
    # parse options
    list=
    output_dir=
    refresh=
    custom_rpath=
    while [ $# -gt 0 ]; do
        case "$1" in
            -h|--help|--usage)
                usage
                quit 0
                ;;
            -v|--version)
                echo "binary_dylib_tool $version"
                quit 0
                ;;
            -l|--list)
                list=1
                shift
                ;;
            -o|--output)
                output_dir="$2"
                shift 2
                ;;
            -r|--refresh)
                refresh=1
                shift
                ;;
            --rpath)
                custom_rpath="$2"
                shift 2
                ;;
            *)
                break
                ;;
        esac
    done

    if [ $# -ne 1 ]; then
        usage
        quit 1
    fi

    mktmp

        # Handle refresh option
    if [ -n "$refresh" ]; then
        echo "Refreshing caches..."
        
        # Clean output directory
        if [ -d "$output_dir" ]; then
            echo "Removing output directory: $output_dir"
            rm -rf "$output_dir"
        fi
        
        # Run cargo clean if we're in a Rust project
        if [ -f "Cargo.toml" ]; then
            echo "Refreshing Rust project..."
            rm target/release/monobean
            cargo build --release
        else
            # Try to find Cargo.toml in parent directories
            current_dir="$(pwd)"
            while [ "$current_dir" != "/" ]; do
                if [ -f "$current_dir/Cargo.toml" ]; then
                    echo "Running cargo clean in $current_dir..."
                    (cd "$current_dir" && rm target/release/monobean && cargo build --release)
                    break
                fi
                current_dir="$(dirname "$current_dir")"
            done
        fi
        
        echo "Cache cleanup completed."
    fi

    binary_file=$(echo "$1" | fully_resolve_links)

    if [ ! -f "$binary_file" ] || [ ! -x "$binary_file" ]; then
        echo "Error: '$1' is not an executable file" >&2
        quit 1
    fi

    # Set default output directory if not specified
    if [ -z "$output_dir" ]; then
        output_dir="$(pwd)/patched_dylibs"
    fi

    # Create absolute path for output directory
    case "$output_dir" in
        /*) 
            # Already absolute
            ;;
        *)
            # Make relative path absolute
            output_dir="$(pwd)/$output_dir"
            ;;
    esac

    set -- "$binary_file"

    OLDIFS=$IFS
    IFS='
'
    # Process the binary file - no need to add it again since it's already in the list
    # The loop below was causing duplication
    IFS=$OLDIFS

    frameworks="$output_dir"

    mkdir -p "$frameworks"

    scan_libs "$@" | sort -u | \
    while read lib; do
        if [ -n "$list" ]; then
            echo "$lib"
        else
            resolved=$(echo "$lib" | fully_resolve_links)
            resolved_basename=${resolved##*/}
            if [ ! -f "$frameworks/$resolved_basename" ]; then
                # Copy the library file
                if [ -f "$resolved" ]; then
                    cp -f "$resolved" "$frameworks" 2>/dev/null
                    if [ $? -eq 0 ]; then
                        echo "Copied: $resolved_basename"
                    else
                        echo "Warning: Failed to copy: $resolved" >&2
                    fi
                else
                    echo "Warning: Library not found: $resolved" >&2
                fi
            fi
            if [ "$resolved" != "$lib" ]; then
                lib_basename=${lib##*/}
                if [ ! -f "$frameworks/$lib_basename" ] && [ -f "$frameworks/$resolved_basename" ]; then
                    ln -s "$resolved_basename" "$frameworks/$lib_basename"
                fi
            fi
        fi
    done

    # Calculate relative path from binary to dylib directory
    binary_dir=$(dirname "$binary_file")
    rel_path=$(get_relative_path "$binary_dir" "$frameworks")
    
    # fix dynamic link info in executables and just copied libs
    [ -z "$list" ] && relink_all "$@" "$rel_path" "$custom_rpath"

    quit 0
}

# Function to calculate relative path from source to target
get_relative_path() {
    source_dir="$1"
    target_dir="$2"
    
    # Convert to absolute paths
    source_abs=$(cd "$source_dir" && pwd)
    target_abs=$(cd "$target_dir" && pwd)
    
    # If target is under source, return relative path
    case "$target_abs" in
        "$source_abs"/*)
            echo "${target_abs#$source_abs/}"
            return
            ;;
    esac
    
    # Calculate common prefix
    common=""
    remaining_source="$source_abs"
    remaining_target="$target_abs"
    
    while [ "$remaining_source" != "/" ] && [ "$remaining_target" != "/" ]; do
        if [ "$remaining_source" = "$remaining_target" ]; then
            common="$remaining_source"
            break
        fi
        
        case "$remaining_target" in
            "$remaining_source"/*)
                common="$remaining_source"
                break
                ;;
        esac
        
        remaining_source=$(dirname "$remaining_source")
    done
    
    # Calculate relative path
    if [ -n "$common" ] && [ "$common" != "/" ]; then
        # Remove common prefix from both paths
        source_rel="${source_abs#$common/}"
        target_rel="${target_abs#$common/}"
        
        # Count directory levels to go up
        up_levels=$(echo "$source_rel" | tr '/' '\n' | grep -c .)
        
        # Build relative path
        rel_path=""
        i=0
        while [ $i -lt $up_levels ]; do
            rel_path="../$rel_path"
            i=$((i + 1))
        done
        
        echo "$rel_path$target_rel"
    else
        # Fallback to absolute path if no common prefix
        echo "$target_abs"
    fi
}

mktmp() {
    tmp="/tmp/third_party_libs_tool_$$"
    mkdir "$tmp" || quit 1
    chmod 700 "$tmp" 2>/dev/null
    trap "quit 1" PIPE HUP INT QUIT ILL TRAP KILL BUS TERM
}

quit() {
    [ -n "$tmp" ] && rm -rf "$tmp" 2>/dev/null
    exit ${1:-0}
}

scan_libs() {
    scratch_dir="$tmp/lib_scan"
    mkdir -p "$scratch_dir"

    lib_scan "$@"

    rm -rf "$scratch_dir"
}

lib_scan() {
    for bin in "$@"; do
        case "$bin" in
            *.dylib)
                ;;
            *)
                [ ! -x "$bin" ] && continue
                ;;
        esac

        # Remove path parts for mark file
        bin_mark_file=$(echo "$bin" | sed 's,/,_,g')

        # if binary is already processed, continue
        [ -d "$scratch_dir/$bin_mark_file" ] && continue

        # otherwise mark it processed
        mkdir -p "$scratch_dir/$bin_mark_file"

        set --

        OLDIFS=$IFS
        IFS='
'
        for lib in $(otool -L "$bin" 2>/dev/null \
              | sed -E '1d; s/^[[:space:]]*//; \,^(/System|/usr/lib),d; s/[[:space:]]+\([^()]+\)[[:space:]]*$//'); do

            [ "$lib" = "$bin" ] && continue

            # check for libs already linked as @rpath/ which usually means /usr/local/lib/
            case "$lib" in
                '@rpath/'*)
                    lib='/usr/local/lib'"${lib#@rpath}"
                    ;;
                '@loader_path/../../../../'*)
                    lib='/usr/local/'"${lib#@loader_path/../../../../}"
                    ;;
                '@loader_path/'*)
                    lib='/usr/local/lib'"${lib#@loader_path}"
                    ;;
            esac

            echo "$lib"
            set -- "$@" "$lib"
        done
        IFS=$OLDIFS

        # recurse
        [ $# -ne 0 ] && lib_scan "$@"
    done
}

fully_resolve_links() {
    while read -r file; do
        while [ -h "$file" ]; do
            file=$(readlink -f "$file")
        done
        echo "$file"
    done
}

lock() {
    mkdir -p "$lock_dir/$1"
}

unlock() {
    rm -rf "$lock_dir/$1"
}

wait_lock() {
    while [ -d "$lock_dir/$1" ]; do
        /bin/bash -c 'sleep 0.1'
    done
}

relink_all() {
    # Store all arguments
    _args="$*"
    # Get the last argument (custom rpath, if provided)
    custom_rpath=$(echo "$_args" | sed 's/.* //')
    # Get the second-to-last argument (relative path)
    rel_path=$(echo "$_args" | sed 's/ [^ ]*$//' | sed 's/.* //')
    # Get all but the last two arguments (executables)
    _exes=$(echo "$_args" | sed 's/ [^ ]*$//' | sed 's/ [^ ]*$//')
    
    # Reset positional parameters to just the executables
    set -- $_exes
    
    lock_dir="$tmp/locks"

    find "$frameworks" -name '*.dylib' > "$tmp/libs"

    for exe in "$@"; do (
        # dylib search path for executable
        wait_lock "$exe"
        lock "$exe"
        
        echo "Processing executable: $exe"
        
        # Make executable writable
        chmod u+w "$exe" 2>/dev/null || true
        
        # Determine which rpath to use
        if [ -n "$custom_rpath" ] && [ "$custom_rpath" != "." ]; then
            # Use custom rpath if provided
            rpath_value="$custom_rpath"
            echo "Using custom rpath: $rpath_value"
        else
            # Use calculated relative path
            rpath_value="@executable_path/$rel_path"
            echo "Using calculated rpath: $rpath_value"
        fi
        
        # Try to add rpath first (before removing signature)
        if install_name_tool -add_rpath "$rpath_value" "$exe"; then
            echo "Added rpath to executable: $exe"
        else
            echo "Note: Could not add rpath to executable: $exe (may already exist)" >&2
        fi
        
        # Remove signature safely after modification
        safe_remove_signature "$exe"
        
        # Re-sign the executable
        safe_add_signature "$exe"
        
        unlock "$exe"

        OLDIFS=$IFS
        IFS='
'
        set --
        for lib in $(cat "$tmp/libs"); do
            set -- "$@" "$lib"
        done
        IFS=$OLDIFS

        for lib in "$@"; do
            wait_lock "$lib"
            lock "$lib"

            echo "Processing library: ${lib##*/}"

            # Make lib writable
            chmod u+w "$lib" 2>/dev/null || true

            # Change id of lib first (before removing signature)
            if install_name_tool -id "@rpath/${lib##*/}" "$lib"; then
                echo "Updated library ID: ${lib##*/}"
            else
                echo "Warning: Failed to update library ID: ${lib##*/}" >&2
            fi

            # Set search path of lib
            if install_name_tool -add_rpath "$rpath_value" "$lib"; then
                echo "Added rpath to library: ${lib##*/}"
            else
                echo "Note: Could not add rpath to library: ${lib##*/} (may already exist)" >&2
            fi

            # Remove signature safely after modification
            safe_remove_signature "$lib"
            
            # Re-sign the library
            safe_add_signature "$lib"

            unlock "$lib"

            # relink executable and all other libs to this lib
            for target in "$exe" "$@"; do (
                relink "$lib" "$target"
            ) & done
            wait
        done
    ) & done
    wait

    rm -rf "$tmp/libs" "$lock_dir"
}

relink() {
    lib=$1
    target=$2

    lib_basename=${lib##*/}
    lib_basename_unversioned_re=$(echo "$lib_basename" | sed 's/\.[0-9.-]*\.dylib$//; s/\./\\./g')

    # remove full path and version of lib in executable
    lib_link_path=$(
        otool -l "$target" 2>/dev/null | \
        sed -n 's,^ *name \(.*/*'"$lib_basename_unversioned_re"'\.[0-9.-]*\.dylib\) (offset .*,\1,p' | \
          head -1
    )

    [ -z "$lib_link_path" ] && return 0

    # check that the shorter basename is the prefix of the longer basename
    # that is, the lib versions match
    lib1=${lib_basename%.dylib}
    lib2=${lib_link_path##*/}
    lib2=${lib2%.dylib}

    if [ "${#lib1}" -le "${#lib2}" ]; then
        shorter=$lib1
        longer=$lib2
    else
        shorter=$lib2
        longer=$lib1
    fi

    case "$longer" in
        "$shorter"*)
            # and if so, relink target to the lib
            wait_lock "$lib"
            lock "$lib"

            # Make target writable
            chmod u+w "$target" 2>/dev/null || true
            
            # Try to change the library reference first (before removing signature)
            if install_name_tool -change "$lib_link_path" "@rpath/$lib_basename" "$target"; then
                echo "Relinked ${target##*/} -> ${lib_basename}"
            else
                echo "Warning: Failed to relink ${target##*/} -> ${lib_basename}" >&2
            fi
            
            # Remove signature safely after modification
            safe_remove_signature "$target"
            
            # Re-sign the target
            safe_add_signature "$target"

            unlock "$lib"
            ;;
    esac
}

# try with sudo in case it fails,
# also suppress duplicate path errors
install_name_tool() {
    out_file="$tmp/install_name_tool.out"

    if ! command install_name_tool "$@" >"$out_file" 2>&1; then
        if grep -Eq -i 'permission  denied|bad file descriptor' "$out_file"; then
            if ! command sudo install_name_tool "$@"; then
                return 1
            fi
        elif grep -Eq -i 'file not in an order that can be processed|link edit information does not fill' "$out_file"; then
            echo "Warning: Skipping corrupted binary file during install_name_tool operation" >&2
            return 1
        elif grep -Eq -i 'would duplicate path' "$out_file"; then
            # This is expected when path already exists, treat as success
            return 0
        else
            cat "$out_file" >&2
            return 1
        fi
    else
        # Command succeeded, but check for warnings that we should ignore
        if grep -Eq -i 'warning.*changes being made.*invalidate.*code signature' "$out_file"; then
            # This is just a warning about code signature, not a failure
            return 0
        fi
    fi

    return 0
}

# Check if a binary file is valid and can be processed
main "$@"

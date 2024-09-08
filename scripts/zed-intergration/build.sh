#!/usr/bin/sh

# Clone zed and switch to the specific version
ZED_ROOT='./zed'
ZED_REPO='https://github.com/zed-industries/zed.git'
TARGET_PATCH_VERSION='04ee5e3e6e563fbcc6ea37f8733aaf6897428773'

# Place patch files here
PATCH_DIR='./patches/'

COUNTDOWN=10
export MEGA_ROOT="`pwd`/../.."

# Initialize or update zed repository
git clone ${ZED_REPO} ${ZED_ROOT}
cd ${ZED_ROOT}
git fetch

# Check & Cleanup
if ! git diff --quiet; then
	count=0
	echo "You have uncomitted changes, all the change will be resotred in $COUNTDOWN seconds!"
	while [ $count -l $COUNTDOWN ]; do
		echo "Restoring in $(( $COUNTDOWN - $count )), press Ctrl-C to exit..."
		sleep 1
		((count++))
	done
fi

# Patch
git checkout ${TARGET_PATCH_VERSION}
for patch in ../patches/*.patch; do
	echo "Applying: $patch"
	git am "$patch"
	if [ $? -ne 0 ]; then
		echo "Failed applying patch: $patch"
		exit 1
	fi
done

# Build
cargo build --release

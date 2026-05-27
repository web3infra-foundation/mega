#!/bin/bash
# Build a custom Debian image with buck2 and Rust toolchain pre-installed.
#
# Pre-installs Rust toolchain, apt packages, and buck2 directly into the image
# via chroot. No VM boot needed, so this is significantly faster than the
# previous cloud-init based approach.
#
# Usage: sudo ./build-custom-image.sh
#
# Note: Must run as root because qemu-nbd / mount / chroot need it.

set -eo pipefail

# ============================================================================
# Configuration
# ============================================================================
OUTPUT_DIR="${OUTPUT_DIR:-$HOME/.local/share/qlean/images}"
IMAGE_NAME="debian-13-buck2"
IMAGE_DIR="$OUTPUT_DIR/$IMAGE_NAME"
BASE_DIR="$OUTPUT_DIR/debian-13-generic-amd64"
BASE_IMAGE="$BASE_DIR/debian-13-generic-amd64.qcow2"
CUSTOM_IMAGE="$IMAGE_DIR/$IMAGE_NAME.qcow2"

IMAGE_SIZE="15G"

RUST_VERSION="1.95.0"
RUST_ARCH="x86_64-unknown-linux-gnu"
RUST_TARBALL="/tmp/rust-${RUST_VERSION}-${RUST_ARCH}.tar.gz"
RUST_TARBALL_URL="https://static.rust-lang.org/dist/rust-${RUST_VERSION}-${RUST_ARCH}.tar.gz"

BUCK2_VERSION="2026-04-15"
BUCK2_ARCH="x86_64-unknown-linux-musl"
BUCK2_URL="https://github.com/facebook/buck2/releases/download/${BUCK2_VERSION}/buck2-${BUCK2_ARCH}.zst"

ROOT_SSH_KEY="ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIF9LTEGIaaad0XP4qUfBoVRgeOg+G36jIWIiqIWP/k4g"

NBD_DEV="/dev/nbd0"
NBD_PART="${NBD_DEV}p1"

# ============================================================================
# Cleanup trap — always release mounts and NBD even on failure / Ctrl-C
# ============================================================================
MOUNT_DIR=""
cleanup() {
    local rc=$?
    set +e
    if [ -n "$MOUNT_DIR" ]; then
        for sub in proc sys dev; do
            mountpoint -q "$MOUNT_DIR/$sub" && sudo umount "$MOUNT_DIR/$sub"
        done
        mountpoint -q "$MOUNT_DIR" && sudo umount "$MOUNT_DIR"
        [ -d "$MOUNT_DIR" ] && rmdir "$MOUNT_DIR" 2>/dev/null
    fi
    sudo qemu-nbd --disconnect "$NBD_DEV" 2>/dev/null
    if [ $rc -ne 0 ]; then
        echo "[build-custom-image] FAILED with exit code $rc"
    fi
    exit $rc
}
trap cleanup EXIT INT TERM HUP

# ============================================================================
# Pre-flight: validate base image, locate kernel/initrd via glob
# ============================================================================
echo "[build-custom-image] Starting custom image build..."

if [ ! -f "$BASE_IMAGE" ]; then
    echo "[build-custom-image] ERROR: Base image not found at $BASE_IMAGE"
    exit 1
fi

KERNEL=$(ls "$BASE_DIR"/vmlinuz-* 2>/dev/null | head -n1 || true)
INITRD=$(ls "$BASE_DIR"/initrd.img-* 2>/dev/null | head -n1 || true)
if [ ! -f "$KERNEL" ]; then
    echo "[build-custom-image] ERROR: No vmlinuz-* found in $BASE_DIR"
    exit 1
fi
if [ ! -f "$INITRD" ]; then
    echo "[build-custom-image] ERROR: No initrd.img-* found in $BASE_DIR"
    exit 1
fi
echo "[build-custom-image] Using kernel: $(basename "$KERNEL")"
echo "[build-custom-image] Using initrd: $(basename "$INITRD")"

mkdir -p "$IMAGE_DIR"

# ============================================================================
# Stage 1: Copy + resize qcow2
# ============================================================================
echo "[build-custom-image] Copying base image..."
cp "$BASE_IMAGE" "$CUSTOM_IMAGE"

echo "[build-custom-image] Resizing image to $IMAGE_SIZE..."
qemu-img resize "$CUSTOM_IMAGE" "$IMAGE_SIZE"

# ============================================================================
# Stage 2: Download Rust on host (avoids needing DNS inside chroot)
# ============================================================================
if [ ! -f "$RUST_TARBALL" ]; then
    echo "[build-custom-image] Downloading Rust ${RUST_VERSION}..."
    curl -fsSL -o "$RUST_TARBALL" "$RUST_TARBALL_URL"
    echo "[build-custom-image] Rust tarball downloaded: $(du -sh "$RUST_TARBALL" | cut -f1)"
else
    echo "[build-custom-image] Using cached Rust tarball: $(du -sh "$RUST_TARBALL" | cut -f1)"
fi

# ============================================================================
# Stage 3: NBD attach + filesystem grow + mount
# ============================================================================
wait_for() {
    # wait_for <path> <max-tries-of-0.25s>
    local path="$1" tries="${2:-40}" i
    for ((i = 0; i < tries; i++)); do
        [ -e "$path" ] && return 0
        sleep 0.25
    done
    return 1
}

if [ ! -e "$NBD_DEV" ]; then
    echo "[build-custom-image] Loading NBD kernel module..."
    sudo modprobe nbd max_part=8
    if ! wait_for "$NBD_DEV" 20; then
        echo "[build-custom-image] ERROR: $NBD_DEV not found after loading module"
        exit 1
    fi
fi

echo "[build-custom-image] Connecting NBD device..."
sudo qemu-nbd --disconnect "$NBD_DEV" 2>/dev/null || true
sudo qemu-nbd -c "$NBD_DEV" "$CUSTOM_IMAGE"

sudo udevadm settle
if ! wait_for "$NBD_PART" 40; then
    echo "[build-custom-image] ERROR: $NBD_PART did not appear"
    exit 1
fi

echo "[build-custom-image] Extending partition to fill $IMAGE_SIZE..."
sudo growpart "$NBD_DEV" 1
sudo udevadm settle
wait_for "$NBD_PART" 20 || true

echo "[build-custom-image] Running e2fsck..."
sudo e2fsck -fy "$NBD_PART"

echo "[build-custom-image] Extending filesystem..."
sudo resize2fs "$NBD_PART"

MOUNT_DIR=$(mktemp -d /tmp/custom-image-mnt.XXXXXX)
echo "[build-custom-image] Mounting image at $MOUNT_DIR..."
sudo mount "$NBD_PART" "$MOUNT_DIR"

# Bind mounts for chroot
sudo mount --bind /proc "$MOUNT_DIR/proc"
sudo mount --bind /sys "$MOUNT_DIR/sys"
sudo mount --bind /dev "$MOUNT_DIR/dev"
sudo cp --remove-destination /etc/resolv.conf "$MOUNT_DIR/etc/resolv.conf"

# Copy Rust tarball into the image's /tmp before entering chroot
sudo cp "$RUST_TARBALL" "$MOUNT_DIR/tmp/rust.tar.gz"

# ============================================================================
# Stage 4: Install everything inside the chroot (no VM boot needed)
# ============================================================================
echo "[build-custom-image] Installing toolchain into image (chroot)..."
sudo BUCK2_URL="$BUCK2_URL" BUCK2_VERSION="$BUCK2_VERSION" \
     ROOT_SSH_KEY="$ROOT_SSH_KEY" \
     chroot "$MOUNT_DIR" /bin/bash <<'CHROOT_EOF' 2>&1 | tee /tmp/chroot_install.log
set -eo pipefail
export HOME=/root
export DEBIAN_FRONTEND=noninteractive
export PATH=/root/.cargo/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin

# Block daemons from starting during apt-get install inside the chroot.
cat > /usr/sbin/policy-rc.d <<'POLICY_EOF'
#!/bin/sh
exit 101
POLICY_EOF
chmod +x /usr/sbin/policy-rc.d

echo "=== [chroot] Disk space before install ==="
df -h /

echo "=== [chroot] Extracting Rust toolchain ==="
tar -xzf /tmp/rust.tar.gz -C /tmp/
bash /tmp/rust-*/install.sh --destdir="" --prefix=/root/.cargo --without=rust-docs
rm -rf /tmp/rust.tar.gz /tmp/rust-*

ln -sf /root/.cargo/bin/rustc /usr/local/bin/rustc
ln -sf /root/.cargo/bin/cargo /usr/local/bin/cargo
echo "=== [chroot] Rust version: $(/root/.cargo/bin/rustc --version) ==="

echo "=== [chroot] Installing apt packages ==="
apt-get update
apt-get install -y \
    clang lld pkg-config protobuf-compiler zstd fuse curl git \
    seccomp libseccomp-dev libpython3-dev openssl libssl-dev build-essential

echo "=== [chroot] Verifying installed tools ==="
git --version
clang --version | head -1
ld.lld --version | head -1
protoc --version
zstd --version | head -1

echo "=== [chroot] Installing buck2 (${BUCK2_VERSION}) ==="
curl -fsSL -o /tmp/buck2.zst "$BUCK2_URL"
zstd -d /tmp/buck2.zst -o /usr/local/bin/buck2
chmod +x /usr/local/bin/buck2
rm -f /tmp/buck2.zst
/usr/local/bin/buck2 --version

echo "=== [chroot] Installing SSH key for root ==="
mkdir -p /root/.ssh
chmod 700 /root/.ssh
echo "$ROOT_SSH_KEY" > /root/.ssh/authorized_keys
chmod 600 /root/.ssh/authorized_keys

echo "=== [chroot] Cleaning apt + temp files ==="
apt-get clean
rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*
rm -f /usr/sbin/policy-rc.d

echo "=== [chroot] Clearing cloud-init state ==="
rm -rf /var/lib/cloud/data/* /var/lib/cloud/instance/* 2>/dev/null || true

# Zero unused blocks so qemu-img convert below can drop them, producing a
# much smaller final qcow2.
echo "=== [chroot] Zeroing free space (helps qcow2 compact) ==="
dd if=/dev/zero of=/EMPTY bs=4M status=none || true
sync
rm -f /EMPTY
sync

echo "=== [chroot] Disk space after install ==="
df -h /
CHROOT_EOF

# ============================================================================
# Stage 5: Unmount, disconnect, then compact the qcow2
# ============================================================================
echo "[build-custom-image] Unmounting chroot bind mounts..."
sudo umount "$MOUNT_DIR/proc"
sudo umount "$MOUNT_DIR/sys"
sudo umount "$MOUNT_DIR/dev"

echo "[build-custom-image] Unmounting image..."
sudo umount "$MOUNT_DIR"
rmdir "$MOUNT_DIR"
MOUNT_DIR=""

sudo qemu-nbd --disconnect "$NBD_DEV"

UNCOMPACT_SIZE=$(du -sh "$CUSTOM_IMAGE" | cut -f1)
echo "[build-custom-image] Compacting qcow2 (size before: $UNCOMPACT_SIZE)..."
qemu-img convert -O qcow2 -c "$CUSTOM_IMAGE" "${CUSTOM_IMAGE}.compact"
mv "${CUSTOM_IMAGE}.compact" "$CUSTOM_IMAGE"
echo "[build-custom-image] Final image size: $(du -sh "$CUSTOM_IMAGE" | cut -f1)"

# ============================================================================
# Stage 6: Copy kernel + initrd, write checksums
# ============================================================================
echo "[build-custom-image] Copying kernel and initrd..."
cp "$KERNEL" "$IMAGE_DIR/"
cp "$INITRD" "$IMAGE_DIR/"

echo "[build-custom-image] Calculating checksums..."
(
    cd "$IMAGE_DIR"
    sha256sum "$IMAGE_NAME.qcow2" > checksums
)

# ============================================================================
# Stage 7: Publish image to the path qlean expects (parent-dir flat layout)
# ============================================================================
# qlean reads $OUTPUT_DIR/$IMAGE_NAME.json which carries `path` pointing at
# $OUTPUT_DIR/$IMAGE_NAME.qcow2 (flat). The subdir build artifacts above are
# kept for record but qlean won't read them directly.
PUBLISHED_IMAGE="$OUTPUT_DIR/$IMAGE_NAME.qcow2"
PUBLISHED_JSON="$OUTPUT_DIR/$IMAGE_NAME.json"

# Refuse to overwrite if a VM has the file locked (qemu holds a write lock).
if [ -f "$PUBLISHED_IMAGE" ] && ! qemu-img info "$PUBLISHED_IMAGE" >/dev/null 2>&1; then
    echo "[build-custom-image] WARNING: $PUBLISHED_IMAGE appears locked (VM running?)."
    echo "[build-custom-image] Skipping publish. Shut down any VMs using it and re-run, or manually:"
    echo "[build-custom-image]   cp $CUSTOM_IMAGE $PUBLISHED_IMAGE"
else
    echo "[build-custom-image] Publishing to $PUBLISHED_IMAGE..."
    cp "$CUSTOM_IMAGE" "$PUBLISHED_IMAGE"

    NEW_DIGEST=$(sha256sum "$PUBLISHED_IMAGE" | awk '{print $1}')

    if [ -f "$PUBLISHED_JSON" ]; then
        echo "[build-custom-image] Updating digest in $PUBLISHED_JSON..."
        TMP_JSON=$(mktemp)
        jq --arg d "$NEW_DIGEST" \
           --arg p "$PUBLISHED_IMAGE" \
           '.path = $p | .digest = ["Sha256", $d]' \
           "$PUBLISHED_JSON" > "$TMP_JSON"
        mv "$TMP_JSON" "$PUBLISHED_JSON"
    else
        echo "[build-custom-image] Creating $PUBLISHED_JSON..."
        cat > "$PUBLISHED_JSON" <<JSON_EOF
{
  "name": "$IMAGE_NAME",
  "path": "$PUBLISHED_IMAGE",
  "arch": "Amd64",
  "distro": "Debian",
  "digest": ["Sha256", "$NEW_DIGEST"]
}
JSON_EOF
    fi
fi

echo ""
echo "[build-custom-image] ==============================================="
echo "[build-custom-image] Custom image build complete!"
echo "[build-custom-image] ==============================================="
echo ""
echo "Build artifacts (subdir):"
echo "  Image:  $CUSTOM_IMAGE"
echo "  Kernel: $IMAGE_DIR/$(basename "$KERNEL")"
echo "  Initrd: $IMAGE_DIR/$(basename "$INITRD")"
echo ""
echo "Published to qlean (flat):"
echo "  Image:  $PUBLISHED_IMAGE"
echo "  JSON:   $PUBLISHED_JSON"
echo ""
echo "sha256:$NEW_DIGEST"
echo ""
cat "$IMAGE_DIR/checksums"

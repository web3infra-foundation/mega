This project automates the creation of a self-customized Linux From Scratch (LFS) system, tailored specifically to serve as the build environment for the mega project. The system leverages the `Buck2` build system to ensure efficient and reproducible builds.

The final output of this process is a qcow2 disk image, which can be used as a virtual machine disk for qemu, providing a pre-configured development environment for the `mega` project.

## Prerequisites

### Prepare on host machine

1. install required packages(use archlinux as example)

```bash
# basic packages
sudo pacman -S base-devel
# qemu
sudo pacman -S qemu-img nbd
```

2. create & mount the build directory

This is the directory where the LFS system will be built and the final disk image will be stored.

```bash
# create a new image,at least 30G for image build, more for mega build & test.
qemu-img create -f qcow2 vda.qcow2 100G


# enable nbd
sudo modprobe nbd max_part=16

export NBDX=/dev/nbd1 # or any other free nbd device
export LFS_DIR=/mnt/lfs

# connect the image to nbd
sudo qemu-nbd -c $NBDX ./vda.qcow2

# create partitions (maybe need wait a few seconds to let the device ready)
sudo parted $NBDX mklabel msdos
sudo parted $NBDX mkpart primary fat32 0% 200M
sudo parted $NBDX mkpart primary ext4 200M 100%

# format partitions
sudo mkfs.vfat ${NBDX}p1
sudo mkfs.ext4 ${NBDX}p2

# mount partitions
sudo mkdir -pv $LFS_DIR
sudo mount ${NBDX}p2 $LFS_DIR
sudo mkdir -pv $LFS_DIR/boot
sudo mount ${NBDX}p1 $LFS_DIR/boot
```

### Prepare on build

Make sure you have corectly clone the submodules by `git clone --recurse-submodules`

1. copy `jhalfs` custome config to `jhalfs` directory

```bash
cp -r custom/config/. jhalfs/custom/config/
```

2. copy needed packages to `source` directory(optional)

If the network is not available, you can copy the needed packages to `source` directory. If not, the build process will download the packages from the internet.

```bash
sudo mkdir -pv $LFS_DIR/sources
```

Use `download-sources.sh` to download the needed packages to target directory manually.

```bash
sudo ./download-sources.sh ./package_list.txt $LFS_DIR/sources
```

3. copy system configurations to target directory

```bash
sudo cp -r custom/system/* $LFS_DIR
```

4. **Edit config scripts**

Because the UUID of the partitions is not fixed, you need to modify the `config` scripts in the `jhalfs` directory to match the actual UUID of the partitions you created.

Run `sudo blkid` to get the UUID of the partitions, The Result should be like this:

```
/dev/nbd1p1: SEC_TYPE="msdos" UUID="60F7-8A4C" BLOCK_SIZE="512" TYPE="vfat" PARTUUID="dfd4b6d0-01"
/dev/nbd1p2: UUID="41686c57-192d-4ed8-87a2-7399482c0261" BLOCK_SIZE="4096" TYPE="ext4" PARTUUID="dfd4b6d0-02"
```

-   Edit `jhalfs/custom/config/1101-custom-config-fstab`

replace the UUID , the result should be like this:

```
# UUID of the p2 partition
uuid1=41686c57-192d-4ed8-87a2-7399482c0261
# UUID of the p1 partition
uuid2=60F7-8A4C
```

-   Edit `jhalfs/custom/config/1102-custom-config-grub`

The result may be like this:

```
# UUID of the p1 partition
BOOTUUID=60F7-8A4C
# PARTUUID of the p2 partition
ROOTPARTUUID=dfd4b6d0-02
```

## Build

Run `make` in `jhalfs` directory to start the build process, and config the build options in TUI.
The following options must be set:

-   `BOOK Settings → [*] Add custom tools support (NEW)`
-   `BOOK Settings → Init system/[*] Systemd (NEW)`
-   set `BOOK Settings → Location of local copy (mandatory)` to absolute path of `mega/alfs/lfs-git` directory.
-   `BOOK Settings → XML Source of Book/(X) Local Copy`
-   `General Settings → [*] Run the makefile`
-   set `General Settings → Build directory` to your build directory, the value of `LFS_DIR` in the previous step.
-   If you want to check the source code of the packages `General Settings → [*] Retrieve source files`
-   `Build Settings → Parallelism settings → [*] Use all cores`
-   If you building failuer first time, in the second should select `[*] Rebuild the Makefile (see help)`.

Then save and exit the TUI, continue the build process. The process will take a long time, maybe 2~3 hours, depending on the performance of the host machine.

### clean up

After the build process is complete, you can umount the partitions and disconnect the image from nbd. Then you can use the `vda.qcow2` as a virtual machine disk at any other machine.

```bash
sudo umount $LFS_DIR/boot
sudo umount $LFS_DIR
sudo qemu-nbd -d $NBDX
```

## Usage

You can use `virt-manager` to create a new virtual machine and use the `vda.qcow2` as the disk image. The virtual machine will boot into the LFS system, and you can use it as a build environment for the `mega` project.

Use the root account to login, the default password is empty.

## Acknowledgements

This project builds upon **jhalfs**, an official automation tool for constructing Linux From Scratch (LFS) systems.

-   jhalfs project: [https://www.linuxfromscratch.org/alfs/download.html](https://www.linuxfromscratch.org/alfs/download.html)
-   Linux From Scratch: [https://www.linuxfromscratch.org](https://www.linuxfromscratch.org)

Some scripts and configurations are borrowed from the following projects:

-   rkos-dev: [https://github.com/rkos-dev/rkos-archive-20230925]

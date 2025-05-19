LFS_PATH=/mnt/lfs

sudo umount $LFS_PATH/boot
sudo umount $LFS_PATH
sudo qemu-nbd -d /dev/nbd1
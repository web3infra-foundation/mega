
IMAGE_PATH=./vda.qcow2
LFS_PATH=/mnt/lfs

sudo modprobe nbd max_part=4
sudo qemu-nbd -c /dev/nbd1 $IMAGE_PATH
sleep 2
sudo mount /dev/nbd1p2 $LFS_PATH
sudo mount /dev/nbd1p1 $LFS_PATH/boot


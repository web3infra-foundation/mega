#!/bin/bash
set -e

LFS=/mnt/lfs

# 确保挂载点存在
if [ ! -d "$LFS" ]; then
  echo "错误：$LFS 不存在"
  exit 1
fi

# 挂载虚拟文件系统
mount --types proc /proc $LFS/proc
mount --rbind /sys $LFS/sys
mount --make-rslave $LFS/sys
mount --rbind /dev $LFS/dev
mount --make-rslave $LFS/dev
mount --bind /run $LFS/run
mount --make-slave $LFS/run

# 如果 /dev/shm 是符号链接，创建对应目标目录
if [ -h $LFS/dev/shm ]; then
  mkdir -pv $LFS/$(readlink $LFS/dev/shm)
fi

# 进入 chroot 环境
chroot "$LFS" /usr/bin/env -i \
  HOME=/root                  \
  TERM="$TERM"               \
  PS1='(lfs chroot) \u:\w\$ ' \
  PATH=/bin:/usr/bin:/sbin:/usr/sbin \
  /bin/bash --login

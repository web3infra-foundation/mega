#!/bin/bash

# 确保我们是在退出 chroot 环境后执行的清理操作
if mount | grep -q 'on /mnt/lfs'; then
    echo "Unmounting chroot mounts..."

    # 卸载 /mnt/lfs 相关的文件系统
    # 注意：调整 `/mnt/lfs` 为你的 chroot 挂载目录路径
    umount -R /mnt/lfs/dev/pts
    umount -R /mnt/lfs/dev
    umount -R /mnt/lfs/proc
    umount -R /mnt/lfs/sys
    umount -R /mnt/lfs/tmp
    umount -R /mnt/lfs/run
    umount -R /mnt/lfs/var

    echo "All chroot mounts have been successfully unmounted."
else
    echo "Not inside chroot environment or mounts not found."
fi

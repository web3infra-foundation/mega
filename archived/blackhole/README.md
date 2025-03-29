# blackhole - Creating a clean build environment for base on Arch Linux

## How to run blackhole

1. Install build tools

```bash
sudo pacman -S archiso
```

"Archiso is a highly-customizable tool for building Arch Linux live CD/USB ISO images. The official images are built with Archiso. It can be used as the basis for rescue systems, linux installers or other systems. This wiki article explains how to install Archiso, and how to configure it to control aspects of the resulting ISO image such as included packages and files. Technical requirements and build steps can be found in the official project documentation. Archiso is implemented with a number of bash scripts. The core component of Archiso is the mkarchiso command. Its options are documented in mkarchiso -h and not covered here." - [1]

2. Create a configuration file directory for the build process

```bash
cd blackhole
mkdir -p ./archiso
cd ./archiso
cp -r /usr/share/archiso/configs/releng/* .
cp -r ../airootfs ./
cp ../packages.x86_64 .
sudo mkarchiso -v -w work/ -o out/ .
```

* Use kvm to run the built image to install the host system.

  * The configuration that needs to be confirmed is that three virtual disks must be configured. The first disk has a capacity of 20GB and is used for host system installation. The second disk is 30GB and is used as the target disk. The third disk is selected according to the actual situation. Block disk capacity plus memory capacity greater than 35GB
  * The installer will run automatically within about 5 minutes after the host image starts. After the installer starts, you need to manually select the install option, and you don’t need to set any options.
  * If it does not run automatically or runs incorrectly, execute the command manually: ```archinstall --config user_configuration.json --creds user_credentials.json --disk_layouts user_disk_layout.json```
  * After the installation is complete, you will be prompted whether to enter the chroot environment. Here,  you can choose no and then reboot.
  * After the host environment is installed, you need to manually mount the swap partition.
  * Host username root password root

## References

1. [https://wiki.archlinux.org/title/archiso](https://wiki.archlinux.org/title/archiso)
2. [https://github.com/archlinux/archinstall](https://github.com/archlinux/archinstall)
3. [https://github.com/archlinux/archinstall/wiki/Building-and-Testing](https://github.com/archlinux/archinstall/wiki/Building-and-Testing)

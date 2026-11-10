#!/bin/sh
pmptest -8 -1
sleep 1
rmmod g_file_storage
modprobe ide-disk
modprobe mxc_ide
sleep 1
dd if=/dev/hda of=/tmp/kalle bs=1 count=1
hdparm -d 1 /dev/hda
hdparm -X udma5 /dev/hda

pkill pwr
pkill player
umount /mnt/vf
losetup -d /dev/loop/0
umount /dev/hda1
umount /dev/hda2

mount -t vfat -o noatime,nodiratime /dev/hda1 /mnt/public
mount -t ext3 -o noatime,nodiratime /dev/hda2 /mnt/vf

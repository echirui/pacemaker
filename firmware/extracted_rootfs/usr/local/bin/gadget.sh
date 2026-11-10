#!/bin/sh

umount /dev/hda1
# /dev/hda2 may be mounted twice
umount /dev/hda2
umount /dev/hda2

if mount | grep /dev/hda
then
  echo Error: /dev/hda still mounted
  exit 1
fi

rmmod g_file_storage
rmmod g_ether
modprobe ide-disk
modprobe mxc_ide
#hdparm -d 0 /dev/hda
modprobe isp1504_arc
#modprobe ehci-hcd
modprobe g_file_storage file=/dev/hda1 removable=1 stall=0

#!/bin/sh

#display
#echo modprobe mxcfb_urt7952
#modprobe mxcfb_urt7952

#keyboard
#echo modprobe pmkbd
#modprobe pmkbd

#slider
#echo modprobe ad7142_slider
#modprobe ad7142_slider

#touch pad
#echo modprobe psmouse
#modprobe psmouse
#echo modprobe serio_mxc
#modprobe serio_mxc

#sound
#echo modprobe snd-wm8580
#modprobe snd-wm8580

#mount hdd
echo modprobe ide-disk
modprobe ide-disk
echo modprobe mxc_ide
modprobe mxc_ide
hdparm -d 1 /dev/hda
hdparm -X udma5 /dev/hda
sleep 1
mkdir -p /mnt/hdd
mount -t vfat /dev/hda1 /mnt/hdd

#usb
#modprobe isp1504_arc
#modprobe ehci-hcd
#modprobe g_file_storage file=/dev/ram0

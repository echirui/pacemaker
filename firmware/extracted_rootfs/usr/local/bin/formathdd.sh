#!/bin/sh

SCRIPT_DIR=`dirname $0`
cd $SCRIPT_DIR

rmmod g_file_storage
modprobe ide-disk
modprobe mxc_ide
sleep 1
fdisk /dev/hda < fdisk_hdd

mkdosfs -F 32 -n Pacemaker /dev/hda1

mkfs -t ext3 /dev/hda2

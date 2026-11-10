#!/bin/sh

SCRIPT_DIR=`dirname $0`
cd $SCRIPT_DIR

rmmod g_file_storage
modprobe ide-disk
modprobe mxc_ide

fdisk /dev/hda < fdisk_swap

BLOCKS=`fdisk -s /dev/hda1`
mkdosfs -F 32 -n Pacemaker /dev/hda1 $BLOCKS

mkfs -t ext3 /dev/hda2

mkswap /dev/hda3

#!/bin/sh

# $1 sw image (zImage or rootfs.jffs2)

. swConfig
. swGetStatus

IMAGE=$1

if [ $IMAGE = "zImage" ]; then

		if [ $BANK = 'B' ]; then
			echo "Erasing flash..."
			flash_eraseall $KERNELBLOCKA 
			echo "Programming..."
			cat $IMAGE > $KERNELBLOCKA
			echo "OK"
		
		else
			echo "Erasing flash..."
			flash_eraseall $KERNELBLOCKAB 
			echo "Programming..."
			cat $IMAGE > $KERNELBLOCKB
			echo "OK"
		fi
	
elif [ $IMAGE = "rootfs.jffs2" ]; then
		if [ $BANK = 'B' ]; then
			echo "Erasing flash..."
			flash_eraseall $ROOTFSBLOCKA 
			echo "Programming..."
			cat $IMAGE > $ROOTFSBLOCKA
			echo "OK"
		
		else
			echo "Erasing flash..."
			flash_eraseall $ROOTFSBLOCKAB 
			echo "Programming..."
			cat $IMAGE > $ROOTFSBLOCKB
			echo "OK"
		fi

else

	echo "incorrect Filename, should be zImage or rootfs.jffs2"
fi



if [ $BANK = 'B' ]; then

	swSetStatus.sh A 1 0
else
	swSetStatus.sh B 1 0
fi


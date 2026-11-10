#!/bin/sh

# $1 path to boot
# $2 kernel/rootfs : 0->excluded, 1->included
# $3 bootloader    : 0->excluded, 1->included 

TEMPDIR=""
. /usr/local/bin/swConfig

#create temp dir
#[ -d $TEMPDIR ] && rm -r $TEMPDIR
#mkdir $TEMPDIR


imageUpdate()
{
	
	. /usr/local/bin/swGetStatus
	
	#keep track components to install
	KERNELIMAGEFLAG=0
	ROOTFSIMAGEFLAG=0
	REDBOOTFLAG=0

	if [ $BANK = 'B' ]; then
		echo "Using flash bank A for update"
	else
 		echo "Using flash bank B for update"
	fi

		
	#check for each ingoing component and upgrade

	#kernel
	echo ""
	if [ -f "$1/zImage" ] && [ "$2" = "1" ]; then

		KERNELIMAGEFLAG=1
		echo "Updating kernel image:"
		
		if [ $BANK = 'B' ]; then
			echo "Erasing flash..."
			flash_eraseall $KERNELBLOCKA 
			echo "Programming..."
			cat $1/zImage > $KERNELBLOCKA
			echo "Verifying..."
			if cmp /mnt/system/boot/zImage $KERNELBLOCKA 2>/dev/null | grep differ; then
				echo "Aborting update: Flash programming failure"
				#reboot the system
				/sbin/reboot
			fi
			echo "OK"
		
		else
			echo "Erasing flash..."
			flash_eraseall $KERNELBLOCKB 
			echo "Programming..."
			cat $1/zImage > $KERNELBLOCKB
			echo "Verifying..."
			if cmp $1/zImage $KERNELBLOCKB 2>/dev/null | grep differ; then
			       echo "Aborting update: Flash programming failure"
			       #reboot the system
			       /sbin/reboot
			fi
			echo "OK"
		fi

	fi

	#root file system
	echo ""

	ROOTFS_NAME=0
# 	if [ "$SVN_VERSION" != "$SVN_VERSION_NEW" ]; then

		if [ -f $1/rootfs.jffs2 ]; then
			ROOTFS_NAME="rootfs.jffs2"
		elif [ $1/cramfs.little ]; then
			ROOTFS_NAME="cramfs.little"
		fi	
#	fi	

	if [ "$ROOTFS_NAME" = "rootfs.jffs2" ] || [ "$ROOTFS_NAME" = "cramfs.little" ]; then
	    if [ "$2" = "1" ]; then
		ROOTFSIMAGEFLAG=1
		echo "Updating rootfs image:"
		
		if [ $BANK = 'B' ]; then
			echo "Erasing flash..."
			flash_eraseall $ROOTFSBLOCKA 
			echo "Programming..."
			cat $1/$ROOTFS_NAME > $ROOTFSBLOCKA
			echo "Verifying..."
			if cmp $1/$ROOTFS_NAME $ROOTFSBLOCKA 2>/dev/null | grep differ; then
				echo "Aborting update: Flash programming failure"
				#reboot the system
			       /sbin/reboot
			fi
			echo "OK"
		else
			echo "Erasing flash..."
			flash_eraseall $ROOTFSBLOCKB 
			echo "Programming..."
			cat $1/$ROOTFS_NAME > $ROOTFSBLOCKB
			echo "Verifying..."
			if cmp $1/$ROOTFS_NAME $ROOTFSBLOCKB 2>/dev/null | grep differ; then
				echo "Aborting update: Flash programming failure"
				#reboot the system
			       /sbin/reboot
			fi
			echo "OK"
		fi
	     fi
	fi

	if [ -f "$1/redboot.bin" ] && [ "$3" = "1" ]; then
		REDBOOTFLAG=1
		echo "Updating redboot image:"
		
		echo "Erasing flash..."
		flash_eraseall $REDBOOTBLOCK 
		echo "Programming..."
		cat $1/redboot.bin > $REDBOOTBLOCK
		echo "Verifying..."
		if cmp $1/redboot.bin $REDBOOTBLOCK 2>/dev/null | grep differ; then
		    echo "Aborting update: Flash programming failure"
		    #reboot the system
		    /sbin/reboot
		fi
		echo "OK"
	fi


	echo ""

	
	if [ $KERNELIMAGEFLAG = 1 ] || [ $ROOTFSIMAGEFLAG = 1 ]; then
		echo "System upgraded, reboot needed"
		if [ $BANK = 'B' ]; then
			swSetStatus.sh A 0 1
		else
			swSetStatus.sh B 0 1
		fi

		#reboot the system
		/sbin/reboot
	
	elif [ $REDBOOTFLAG = 1 ]; then
	        #reboot the system
		/sbin/reboot
	else
		
	        echo "Incomplete Image or no FW or BL upgrade"
		sleep 5
		#reboot the system
		/sbin/reboot
	fi

	return
}

#do the update
imageUpdate $1 $2 $3

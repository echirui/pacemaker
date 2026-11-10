#!/bin/sh

# $1 sw image

TEMPDIR=""
. /usr/local/bin/swConfig

#create temp dir
#[ -d $TEMPDIR ] && rm -r $TEMPDIR
#mkdir $TEMPDIR

#status variable
IMAGEUPDATESTATUS=""

#exit function	
abort()
{
	echo "imageUpdate abort: $IMAGEUPDATESTATUS"
}

imageUpdate()
{
	
	. /usr/local/bin/swGetStatus
	
	#keep track components to install
	KERNELIMAGEFLAG=0
	ROOTFSIMAGEFLAG=0
	APPLICATIONFLAG=0

	if [ $BANK = 'B' ]; then
		echo "Using flash bank A for update"
	else
 		echo "Using flash bank B for update"
	fi

	#execute from temp directory
	TARIMAGE=$1	
	TARIMAGE=${TARIMAGE##*/}
	TEMPDIR=$1
	TEMPDIR=${TEMPDIR%%"$TARIMAGE"}
	mv $1 $TEMPDIR
	cd $TEMPDIR

	#calculate and verify checksum
	echo "Verifying image:"
	tar -xf $TARIMAGE checksum $SWFILENAME 2>/dev/null
	rm -f $TARIMAGE

	if [ -e $SWFILENAME ] && [ -e checksum ]; then
	
		echo "Calculating checksum..."
		md5sum $SWFILENAME > chksum
		if cmp checksum chksum 2>/dev/null | grep ""; then		
			echo "Aborting update: Checksum error"
			IMAGEUPDATESTATUS="Checksum error"
			rm -f checksum
			rm -f chksum
			rm -f $SWFILENAME
			abort
			return
		else
			echo ""
		fi
	else
		echo "Aborting update: Invalid Image"
		IMAGEUPDATESTATUS="Invalid Image"
		rm -f checksum
		rm -f $SWFILENAME
		abort
		return
	fi	

	rm -f checksum
	rm -f chksum
	#extract tar file
	echo "Extracting main image:" ${SWFILENAME%.gz}
	gunzip $SWFILENAME
	echo "OK"

	SWFILENAME=${SWFILENAME%.gz}

		
	#check for each ingoing component and upgrade

	#kernel
	echo ""
	if [ `tar -tf $SWFILENAME | grep zImage` ]; then

		KERNELIMAGEFLAG=1
		echo "Updating kernel image:" `tar -xvf $SWFILENAME zImage`
		
		if [ $BANK = 'B' ]; then
			echo "Erasing flash..."
			flash_eraseall $KERNELBLOCKA 
			echo "Programming..."
			cat zImage > $KERNELBLOCKA
			echo "Verifying..."
			if cmp zImage $KERNELBLOCKA 2>/dev/null | grep differ; then
				echo "Aborting update: Flash programming failure"
				IMAGEUPDATESTATUS="Flash programming failure"
				rm -f $SWFILENAME zImage
				abort
				return
			fi
			echo "OK"
		
		else
			echo "Erasing flash..."
			flash_eraseall $KERNELBLOCKB 
			echo "Programming..."
			cat zImage > $KERNELBLOCKB
			echo "Verifying..."
			if cmp zImage $KERNELBLOCKB 2>/dev/null | grep differ; then
				echo "Aborting update: Flash programming failure"
				IMAGEUPDATESTATUS="Flash programming failure"
				rm -f $SWFILENAME zImage
				abort
				return
			fi
			echo "OK"
		fi
		rm -f zImage

	fi

	#root file system
	echo ""

	ROOTFS_NAME=0	
	if [ `tar -tf $SWFILENAME | grep rootfs.jffs2` ]; then
	    ROOTFS_NAME="rootfs.jffs2"
	elif [ `tar -tf $SWFILENAME | grep cramfs.little` ]; then
	    ROOTFS_NAME="cramfs.little"
	fi	
	
	if [ "$ROOTFS_NAME" = "rootfs.jffs2" ] || [ "$ROOTFS_NAME" = "cramfs.little" ]; then
		ROOTFSIMAGEFLAG=1
		echo "Updating rootfs image:" `tar -xvf $SWFILENAME $ROOTFS_NAME`
		
		if [ $BANK = 'B' ]; then
			echo "Erasing flash..."
			flash_eraseall $ROOTFSBLOCKA 
			echo "Programming..."
			cat $ROOTFS_NAME > $ROOTFSBLOCKA
			echo "Verifying..."
			if cmp $ROOTFS_NAME $ROOTFSBLOCKA 2>/dev/null | grep differ; then
				echo "Aborting update: Flash programming failure"
				IMAGEUPDATESTATUS="Flash programming failure"
				rm -f $SWFILENAME $ROOTFS_NAME
				abort
				return
			fi
			echo "OK"
		else
			echo "Erasing flash..."
			flash_eraseall $ROOTFSBLOCKB 
			echo "Programming..."
			cat $ROOTFS_NAME > $ROOTFSBLOCKB
			echo "Verifying..."
			if cmp $ROOTFS_NAME $ROOTFSBLOCKB 2>/dev/null | grep differ; then
				echo "Aborting update: Flash programming failure"
				IMAGEUPDATESTATUS="Flash programming failure"
				rm -f $SWFILENAME $ROOTFS_NAME
				abort
				return
			fi
			echo "OK"
		fi
		rm -f $ROOTFS_NAME

	fi
	#Applications
	echo ""
	if [ `tar -tf $SWFILENAME | grep system.tar.gz` ]; then

		APPLICATIONFLAG=1
		echo "Updating Application:" 
		
	
		echo "Programming..."
		cd /mnt/hdd/system/tmp/
		tar -xvzf $SWFILENAME system.tar.gz
		cd /mnt/hdd
		echo "Erasing..."
		rm -r /mnt/hdd/system
		mv /mnt/hdd/system/tmp/system /mnt/hdd/system 
		echo "OK"
		
		rm -f system.tar.gz
		cd $TEMPDIR

	fi


	#clean up
	rm -f $SWFILENAME

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
	else
		echo "Aborting update: Incomplete Image"
		IMAGEUPDATESTATUS="Incomplete image"
		abort
	fi

	if [ $APPLICATIONFLAG = 1 ]; then
		echo "Application uppgraded"
		#if [ $HDDBANK = 'B' ]; then
		#	echo "A" > /mnt/hdd/system/app/bank
		#else
		#	echo "B" > /mnt/hdd/system/app/bank
		#fi
		#reboot the system
		/sbin/reboot
	fi

	return
}

#do the update
imageUpdate $1

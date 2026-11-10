#!/bin/sh

# $1 sw image


. /usr/local/bin/swGetStatus

IMAGE=$1

if [ "$IMAGE" != "zImage" ]; then
    IMAGE=${IMAGE##*/}
fi

if [ "$IMAGE" = "zImage" ]; then

    if [ $BANK = 'B' ]; then
	echo "Using flash bank B for update"
	flash_eraseall /dev/mtd/3
	cat $1 > /dev/mtd/3
    else
	echo "Using flash bank A for update"
	flash_eraseall /dev/mtd/2
	cat $1 > /dev/mtd/2
    fi
else
    echo "Input error!!"
fi



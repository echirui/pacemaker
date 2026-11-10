#!/bin/sh

#$1 Flash Bank
#$2 Status Flag
#$3 Upgrade Flag

echo "Updating bank/status flags..."
echo $1$2$3$4 > /dev/mtdblock1
. /usr/local/bin/swGetStatus

if [ $1 == $BANK ] && [ $2 == $STATUS ] && [ $3 == $UPGRADE ] && [ $4 == $ERROR ]; then
	echo "Done"
	echo "Flash bank:" $BANK
	echo "Status flag:" $STATUS
	echo "Upgrade flag:" $UPGRADE
    echo "Error flag:" $ERROR
else 
	echo "Error updating status partition"
fi


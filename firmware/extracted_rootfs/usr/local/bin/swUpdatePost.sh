#!/bin/sh

SWUPDATE_PATH=/usr/local/bin

. $SWUPDATE_PATH/swConfig
#examine the current Image status
. $SWUPDATE_PATH/swGetStatus

echo ""
echo "Flash bank:" $BANK
echo "Status flag:" $STATUS
echo "Upgrade flag:" $UPGRADE

echo "Checking Image status..."

if [ $STATUS = '0' ]; then
	#in case of finished sw upgrade
	if [ $UPGRADE = '0' ]; then

		#successful update
		echo "Status: Successful software update"
		if [ $BANK = 'B' ]; then
			$SWUPDATE_PATH/swSetStatus.sh B 1 0
		else	
			$SWUPDATE_PATH/swSetStatus.sh A 1 0
		fi
	else
		#software update failed
		echo "Status: Software update Failed!"
		if [ $BANK = 'B' ]; then
			$SWUPDATE_PATH/swSetStatus.sh B 1 0
		else	
			$SWUPDATE_PATH/swSetStatus.sh A 1 0
		fi
			
		echo "Aborted: Image could not be executed"
	fi
else
	#system is booting as normal, no update in progress
	echo ""
fi






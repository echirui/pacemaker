#!/bin/sh



. /usr/local/bin/swGetStatus


    if [ $BANK = 'B' ]; then
	echo "Switch to Bank A"
	swSetStatus.sh A 0 1
    else
	echo "Switch to Bank B"
	swSetStatus.sh B 0 1
    fi



#!/bin/sh
rmmod g_file_storage
modprobe g_ether host_addr=00:dc:c8:f7:75:05 dev_addr=00:dd:dc:eb:6d:f1
if [ -f "/etc/paceIpAddr" ]; then
    ifconfig usb0 `cat /etc/paceIpAddr`
else
    ifconfig usb0 192.168.17.17
fi

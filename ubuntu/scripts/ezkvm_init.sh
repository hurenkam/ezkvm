# Change permissions for vm-108 drives so ezkvm group members have rw access
for disk in vm-108-tmp vm-108-boot vm-108-efidisk vm-108-tpmstate; do
	lvchange -kn /dev/vm1/$disk
	lvchange -ay /dev/vm1/$disk
	chown root:ezkvm /dev/vm1/$disk
done	

# Change permissions for runtime directories so ezkvm group members have rw access
for dir in /var/run/ezkvm /var/log/ezkvm; do
	chmod -R 775 $dir
	chown -R root:ezkvm $dir
done

# Change permissions for vfio devices so ezkvm group members have rw access
for vfio in 15 16; do
	chmod -R 775 /dev/vfio/$vfio
	chown -R root:ezkvm /dev/vfio/$vfio
done

# Load kvmfr module, and change permissions so ezkvm group members have rw access
modprobe kvmfr
chmod 666 /dev/kvmfr0 
mount /dev/vm0/trixie_root /mnt/trixie/

# Setup nat masquerading for br0 interface
nft add table ip nat
nft 'add chain ip nat postrouting { type nat hook postrouting priority 100; }'
nft add rule ip nat postrouting oifname "eno1" ip saddr 192.168.191.0/24 masquerade


# restore latest known good snapshot
for disk in vm-108-tmp vm-108-boot vm-108-efidisk vm-108-tpmstate; do
	lvremove /dev/vm1/$disk
	lvcreate --snap --name $disk /dev/vm1/snap_${disk}_intermediate_20260503
	lvchange -kn /dev/vm1/$disk
	lvchange -ay /dev/vm1/$disk
	chown root:ezkvm /dev/vm1/$disk
done	


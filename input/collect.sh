cp /etc/pve/storage.cfg .
for file in `(cd /etc/pve/qemu-server/; ls *.conf | sed s/.conf//)`
do cp /etc/pve/qemu-server/$file.conf .
qm showcmd $file > $file.qemu.cmd
done

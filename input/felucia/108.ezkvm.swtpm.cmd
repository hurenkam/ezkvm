/usr/bin/swtpm socket --tpm2 --tpmstate backend-uri=file:///dev/vm1/vm-108-tpmstate --ctrl type=unixio,path=/var/run/ezkvm/wakiza.swtpm,mode=0600 --pid file=/var/run/ezkvm/wakiza.swtpm.pid --terminate --log file=/var/log/ezkvm/wakiza-swtpm.log,level=1 --daemon


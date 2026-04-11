The following errors have been found while attempting to run ezkvm:

1. Problem with QMP Monitor:
   log message:
        qemu-system-x86_64: -qmp unix:/var/run/qemu-server/108.qmp: Failed to connect to '/var/run/qemu-server/108.qmp': No such file or directory

    workaround:
        comment-out 'qmp' section from yaml config file

2. Problem with CDROM Drive:
   log message:
        qemu-system-x86_64: -drive if=none,id=drive-ide2,media=cdrom,format=raw,readonly=on,aio=io_uring: A block device must be specified for "file"
    
    workaround:
        comment-out cdrom section in yaml config file

4. Problem connecting to started vm (wakiza)
    The current wakiza.yaml config seems to be able to start qemu (ezkvm does not exit, and qemu shows up in ps aux | grep qemu, showing it is running), however there is no proof of life on any of the exposed interfaces:

      A. Remote-viewer starts, but shows a 'graphics screen not initialized', this is to be expected as wakiza has gpu passthrough configured, and should be using looking-glass as viewer. It would be better if remote-viewer does not start when gpu-passthrough is configured.

      B. Looking-glass client does not seem to start (or exits immediately), as the screen does not appear, and also ps aux | grep looking does not show any instances.

      C. Network connection is not possible, wakiza should appear at 192.168.191.101, however both ping and remote-desktop over rdp timeout.

      D. When a physical monitor is connected to the gpu output, it remains black, showing no boot screen.

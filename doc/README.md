This directory contains documentation for the ezkvm project:

- dev: Information intended for development
   - requirements: Requirements and specifications for the ezkvm tool
   - domain-knowledge: Collected information about relevant technologies and tools
       - proxmox: Contains expert information about the inner workings of proxmox, typically extracted from the public repo's.
       - qemu: Contains expert information about the inner workings of qemu, typically extracted from the public repo's.
       - Q35: Contains expert information about the Q35 chipset architecture and its inner workings, typically extracted from online sources.
       - i440fx: Contains expert information about the intel 440FX chipset architecture and its inner workings, typically extracted from online sources.
       - windows: Contains expert information about Windows 11, aimed at using best practices, and troubleshooting windows problems w.r.t. Q35 based architecture.
       - linux: Contains expert information about linux, aimed at using best practices and troublshooting problems w.r.t. running linux on Q35 or i440fx chipset based architectures.
       - macos: Contains expert information about linux, aimed at using best practices and troublshooting problems w.r.t. running macos on Q35 based architecture.       
       - gpu: Contains expert information w.t.t. using amd, nvidia, and intel GPU's in windows, linux or macos. Extra focus on troubleshooting GPU passthrough issues for running windows and macos on qemu vm.
       - swtpm: Contains expert information about the inner workings of swtpm, typically extracted from the public repo's.
       - looking-glass: Contains expert information about the inner workings of looking-glass-client, looking-glass-host and the kernel module, typically extracted from the public repo's.

- user: Information for the end-user
   - quick-start
   - how-to
   - reference

- planning: Information about planning and progress
   - backlog: This is where features live while they are being worked on. Tracking of progress also takes place here
   - features: 
       - ideas: This is where ideas for features land, they will typically be unprepared and incomplete
       - prepared: This is where features are moved when they are considered ready for implementation
       - done: This is where features are moved when the implementation is done

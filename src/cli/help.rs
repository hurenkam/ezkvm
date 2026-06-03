use std::collections::HashSet;

pub fn print_help() {
    let mut importers = HashSet::new();
    importers.insert("ezkvm");
    importers.insert("proxmox");
    importers.insert("qemu");
    importers.insert("libvirt");

    let mut importers_list: Vec<&str> = importers.into_iter().collect();
    importers_list.sort_unstable();

    println!("ezkvm CLI syntax");
    println!();
    println!("Usage:");
    println!(
        "  ezkvm --input:type=<importer>,config=<path>[,<importer-args>] [--validate] [--show-runtime] [--output:type=<type>[,path=<file>]]"
    );
    println!(
        "  ezkvm --import:type=<importer>,config=<path>[,<importer-args>] [--validate] [--show-runtime] [--output:type=<type>[,path=<file>]]"
    );
    println!();
    println!("Importers: {}", importers_list.join(", "));
    println!("Output types: qemu, ezkvm");
    println!();
    println!("Examples:");
    println!(
        "  ezkvm --input:type=ezkvm,config=win11-dev.yaml,host=/etc/ezkvm/host.yaml,profiles=/etc/ezkvm/profiles.d --validate"
    );
    println!(
        "  ezkvm --input:type=ezkvm,config=win11-dev.yaml,host=/etc/ezkvm/host.yaml,profiles=/etc/ezkvm/profiles.d --show-runtime"
    );
    println!(
        "  ezkvm --import:type=proxmox,config=108.conf,storage=/etc/pve/storage.cfg --output:type=ezkvm"
    );
    println!();
    println!("Notes:");
    println!("  - Exporters are currently stubbed and write placeholder output files.");
    println!("  - Use --help to show this message.");
}

//! CLI help text rendering for the ezkvm command-line interface.
//!
//! Related documentation:
//! - src/README.md
//! - src/cli/README.md

/// Prints the ezkvm CLI usage, examples, and operational notes.
///
/// This keeps the user-facing help text centralized so command syntax stays
/// consistent with the parser and module documentation.
pub fn print_help() {
    println!("ezkvm CLI syntax");
    println!();
    println!("Usage:");
    println!("  ezkvm import --input.type <type> [input flags]");
    println!("  ezkvm export --output.type <type> [output flags]");
    println!(
        "  ezkvm convert --input.type <type> [input flags] --output.type <type> [output flags]"
    );
    println!("  ezkvm show-runtime --name <name>");
    println!("  ezkvm start --name <name>");
    println!("  ezkvm stop --name <name>");
    println!("  ezkvm reset --name <name>");
    println!("  ezkvm shutdown --name <name>");
    println!();
    println!("Input types: ezkvm, proxmox, qemu, libvirt");
    println!("Output types: qemu, ezkvm, proxmox, libvirt");
    println!();
    println!("Examples:");
    println!(
        "  ezkvm import --input.type ezkvm --input.host /etc/ezkvm/host.yaml --input.vm win11-dev.yaml"
    );
    println!(
        "  ezkvm convert --input.type proxmox --input.storage /etc/pve/storage.cfg --input.vm 108.conf --output.type ezkvm --output.host /etc/ezkvm/host.yaml --output.vm 108.yaml"
    );
    println!(
        "  ezkvm convert --input.type ezkvm --input.host /etc/ezkvm/host.yaml --input.vm win11-dev.yaml --output.type qemu"
    );
    println!(
        "  ezkvm convert --input.type ezkvm --input.host /etc/ezkvm/host.yaml --input.vm win11-dev.yaml --output.type libvirt --output.vm win11-dev.xml"
    );
    println!("  ezkvm show-runtime --name win11-dev");
    println!();
    println!("Notes:");
    println!("  - Exporters currently write the RuntimeConfig snapshot text to the output file.");
    println!("  - 'export' without an import context is not implemented yet; use convert.");
    println!("  - Use --help to show this message.");
}

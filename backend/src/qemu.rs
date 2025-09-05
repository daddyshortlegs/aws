use tokio::process::{Command, Child};

pub fn vm_start(qcow2_file: &str, ssh_port: u16) -> Result<Child, std::io::Error> {
    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.args([
        "-m",
        "8192",
        "-smp",
        "6",
        "-drive",
        &format!("file={}", qcow2_file),
        "-boot",
        "d",
        "-vga",
        "virtio",
        "-netdev",
        &format!("user,id=net0,hostfwd=tcp::{}:-:22", ssh_port),
        "-device",
        "e1000,netdev=net0",
    ]);

    cmd.spawn()
}
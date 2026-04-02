use tokio::process::{Child, Command};
use uuid::Uuid;

pub enum NetworkConfig {
    User { ssh_port: u16 },
    Bridge { mac_address: String },
}

pub fn mac_from_uuid(id: &str) -> String {
    let uuid = Uuid::parse_str(id).expect("valid uuid");
    let b = uuid.as_bytes();
    format!("52:54:00:{:02x}:{:02x}:{:02x}", b[0], b[1], b[2])
}

pub fn vm_start(qcow2_file: &str, network: &NetworkConfig) -> Result<Child, std::io::Error> {
    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.args([
        "-m",
        "8192",
        "-smp",
        "6",
        "-drive",
        &format!("file={qcow2_file}"),
        "-boot",
        "d",
        "-vga",
        "virtio",
        "-nographic",
    ]);

    match network {
        NetworkConfig::User { ssh_port } => {
            cmd.args([
                "-netdev",
                &format!("user,id=net0,hostfwd=tcp::{ssh_port}-:22"),
                "-device",
                "e1000,netdev=net0",
            ]);
        }
        NetworkConfig::Bridge { mac_address } => {
            cmd.args([
                "-netdev",
                "bridge,id=net0,br=br0",
                "-device",
                &format!("e1000,netdev=net0,mac={mac_address}"),
            ]);
        }
    }

    cmd.spawn()
}

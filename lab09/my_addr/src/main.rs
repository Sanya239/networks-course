use if_addrs::get_if_addrs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ifaces = get_if_addrs()?;

    for iface in ifaces {
        if let if_addrs::IfAddr::V4(v4) = iface.addr {
            println!("Interface: {}", iface.name);
            println!("  IP address: {}", v4.ip);
            println!("  Netmask: {}", v4.netmask);
            println!();
        }
    }

    Ok(())
}

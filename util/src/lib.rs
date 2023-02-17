use std::{error::Error, net::{IpAddr, Ipv4Addr}};

use mdns_sd::ServiceDaemon;
use team_number::TeamNumber;

pub mod buffer_reader;
pub mod buffer_writter;
pub mod robot_voltage;
pub mod socket;
pub mod team_number;

pub fn find_robot_ip(
    team_number: impl Into<TeamNumber>,
) -> Result<IpAddr, Box<dyn Error>> {
    let team_number = team_number.into();
    let mdns = ServiceDaemon::new()?;
    // no idea what this service is but it works
    let service = "_ni-rt._tcp.local.";
    let name = format!("roboRIO-{team_number}-FRC.{service}");

    //try for 15 seconds to find the mdns
    let receiver = mdns.browse(service)?;
    receiver.recv_timeout(std::time::Duration::from_secs(5))?;

    for event in receiver.iter() {
        println!("{:?}", event);
        match event {
            mdns_sd::ServiceEvent::ServiceResolved(service) => {
                if service.get_fullname().eq_ignore_ascii_case(&name) {
                    if let Some(found_ip) = service.get_addresses().iter().next() {
                        return Ok(IpAddr::V4(*found_ip));
                    }
                }
            }
            mdns_sd::ServiceEvent::ServiceFound(_, _) => {}
            mdns_sd::ServiceEvent::ServiceRemoved(_, _) => {}
            mdns_sd::ServiceEvent::SearchStopped(_) => break,
            mdns_sd::ServiceEvent::SearchStarted(_) => {}
        }
    }

    if team_number.0 <= 9999 {
        println!("Trying static");
        let upper = (team_number.0 / 100) as u8;
        let lower = (team_number.0 % 100) as u8;
        return Ok(IpAddr::V4(Ipv4Addr::new(10, upper, lower, 22)));
    }

    Err("Cannot find roborio".into())
}

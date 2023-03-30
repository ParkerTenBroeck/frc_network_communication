use std::{
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    sync::Arc,
};

use atomic::Atomic;
use eframe::egui;
use robot_comm::{
    driver_to_robot::DriverstationToRobotCorePacketDate,
    robot_to_driver::RobotToDriverstationPacket,
    util::{
        buffer_reader::{BufferReader, CreateFromBuf},
        buffer_writter::SliceBufferWritter,
    },
};

pub fn run_bruh() {
    let relay = Arc::new(Relay::default());
    let r1 = relay.clone();
    std::thread::spawn(move || {
        let rx = UdpSocket::bind("0.0.0.0:1110").unwrap();
        // let tx = UdpSocket::bind("0.0.0.0:1110").unwrap();
        let tx = SocketAddr::new(Ipv4Addr::new(10, 42, 0, 182).into(), 1110);

        let mut buf = [0u8; 128];
        loop {
            let (rec, from) = rx.recv_from(&mut buf).unwrap();
            rx.send_to(&buf[..rec], tx).unwrap();
            // CreateFromBuf
            let val = DriverstationToRobotCorePacketDate::create_from_buf(&mut BufferReader::new(
                &buf[..rec],
            ));
            if let Ok(val) = val {
                r1.driver_to_robot.store(
                    DriverstationToRobotCorePacketDateWrapper { data: val },
                    atomic::Ordering::Relaxed,
                );
                println!("robot_to_driver: {:#?}", val)
            }
        }
    });

    let r2 = relay.clone();
    std::thread::spawn(move || {
        let rx = UdpSocket::bind("0.0.0.0:1150").unwrap();
        // let tx = UdpSocket::bind("0.0.0.0:1110").unwrap();
        let tx = SocketAddr::new(Ipv4Addr::new(10, 11, 14, 197).into(), 1150);

        let mut buf = [0u8; 128];
        loop {
            let rec = rx.recv(&mut buf).unwrap();
            rx.send_to(&buf[..rec], tx).unwrap();
            // println!("robot to driver: {:?}", &buf[..rec])
            let val =
                RobotToDriverstationPacket::create_from_buf(&mut BufferReader::new(&buf[..rec]));

            if let Ok(val) = val {
                r2.robot_to_driver.store(
                    RobotToDriverstationPacketWrapper { data: val },
                    atomic::Ordering::Relaxed,
                );
                println!("driver_to_robot: {:#?}", val)
            }
        }
    });

    let options = eframe::NativeOptions {
        // initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|_cc| Box::new(MyApp { relay })),
    )
    .unwrap()
}

#[derive(Debug, Default)]
struct Relay {
    driver_to_robot: Atomic<DriverstationToRobotCorePacketDateWrapper>,
    robot_to_driver: Atomic<RobotToDriverstationPacketWrapper>,
}

#[repr(align(8))]
#[derive(Debug, Default, Clone, Copy)]
struct DriverstationToRobotCorePacketDateWrapper {
    data: DriverstationToRobotCorePacketDate,
}

#[repr(align(8))]
#[derive(Debug, Default, Clone, Copy)]
struct RobotToDriverstationPacketWrapper {
    data: RobotToDriverstationPacket,
}

struct MyApp {
    relay: Arc<Relay>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.heading("Driver To Robot");
                    ui.label(format!(
                        "{:#?}",
                        self.relay
                            .driver_to_robot
                            .load(atomic::Ordering::Relaxed)
                            .data
                    ));
                });

                ui.vertical(|ui| {
                    ui.heading("Robot To Driver");
                    ui.label(format!(
                        "{:#?}",
                        self.relay
                            .robot_to_driver
                            .load(atomic::Ordering::Relaxed)
                            .data
                    ));
                });
            });

            ctx.request_repaint();
        });
    }
}

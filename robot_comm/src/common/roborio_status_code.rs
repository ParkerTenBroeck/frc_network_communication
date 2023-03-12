mycelium_bitfield::bitfield! {
    #[derive(Default, Eq, PartialEq, Hash)]
    pub struct RobotStatusCode<u8>{
        /// No idea
        pub const IDK: bool;
        // pub const _2: bool;
        // pub const _3: bool;
        // pub const _4: bool;
        const _RESERVED1 = 3;
        pub const _5: bool;
        pub const ROBOT_HAS_CODE: bool;
        const _RESERVED2 = 2;
        // pub const _7: bool;
        // pub const _8: bool;
    }
}

impl RobotStatusCode {

    pub fn set_has_robot_code(&mut self, has_robot_code: bool) -> &mut Self{
        self.set(Self::ROBOT_HAS_CODE, has_robot_code);
        self
    }

    pub fn has_robot_code(&self) -> bool {
        self.get(Self::ROBOT_HAS_CODE)
    }
}

impl RobotStatusCode {
    pub fn to_bits(&self) -> u8 {
        self.0
    }
}

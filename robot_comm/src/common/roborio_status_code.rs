mycelium_bitfield::bitfield! {
    #[derive(Default, Eq, PartialEq, Hash)]
    pub struct RobotStatusCode<u8>{
        /// No idea
        pub const DISSABLED: bool;
        pub const TELEOP_CODE: bool;
        pub const AUTON_CODE: bool;
        pub const TEST_CODE: bool;
        pub const IS_ROBORIO: bool;
        pub const ROBOT_HAS_CODE: bool;
        pub const _7: bool;
        pub const _8: bool;
    }
}

impl RobotStatusCode {
    pub fn set_has_robot_code(&mut self, has_robot_code: bool) -> &mut Self {
        self.set(Self::ROBOT_HAS_CODE, has_robot_code);
        self
    }

    pub fn has_robot_code(&self) -> bool {
        self.get(Self::ROBOT_HAS_CODE)
    }

    pub fn set_disabled(&mut self) -> &mut Self {
        self.set(Self::DISSABLED, true);
        self.set(Self::TELEOP_CODE, false);
        self.set(Self::TEST_CODE, false);
        self.set(Self::AUTON_CODE, false);
        self
    }

    pub fn set_teleop(&mut self) -> &mut Self {
        self.set(Self::DISSABLED, false);
        self.set(Self::TELEOP_CODE, true);
        self.set(Self::TEST_CODE, false);
        self.set(Self::AUTON_CODE, false);
        self
    }

    pub fn set_test(&mut self) -> &mut Self {
        self.set(Self::DISSABLED, false);
        self.set(Self::TELEOP_CODE, false);
        self.set(Self::TEST_CODE, true);
        self.set(Self::AUTON_CODE, false);
        self
    }

    pub fn observe_robot_autonomus(&mut self) -> &mut Self {
        self.set(Self::DISSABLED, false);
        self.set(Self::TELEOP_CODE, false);
        self.set(Self::TEST_CODE, false);
        self.set(Self::AUTON_CODE, true);
        self
    }

    pub fn set_is_roborio(&mut self, is_roborio: bool) -> &mut Self {
        self.set(Self::IS_ROBORIO, is_roborio);
        self
    }
}

impl RobotStatusCode {
    pub fn to_bits(&self) -> u8 {
        self.0
    }
}

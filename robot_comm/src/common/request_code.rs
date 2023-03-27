mycelium_bitfield::bitfield! {
    #[derive(Default, PartialEq, Eq, Hash)]
    pub struct RobotRequestCode<u8>{
        pub const NORMAL: bool;
        // pub const _2: bool;
        // pub const _3: bool;
        pub const _RESERVED_0 = 2;
        pub const RESTART_ROBORIO: bool;
        pub const RESTART_ROBORIO_CODE: bool;
        pub const _RESERVED_1 = 3;
        // pub const _6: bool;
        // pub const _7: bool;
        // pub const _8: bool;
    }
}

impl RobotRequestCode {
    pub fn is_normal(&self) -> bool {
        self.get(Self::NORMAL)
    }

    pub fn is_invalid(&self) -> bool {
        self.get(Self::_RESERVED_0) > 0 || self.get(Self::_RESERVED_1) > 0
    }

    pub fn should_restart_roborio(&self) -> bool {
        self.get(Self::RESTART_ROBORIO)
    }

    pub fn should_restart_roborio_code(&self) -> bool {
        self.get(Self::RESTART_ROBORIO_CODE)
    }

    pub fn set_normal(&mut self, normal: bool) -> &mut Self {
        self.set(Self::NORMAL, normal);
        self
    }

    pub fn set_restart_roborio(&mut self, restart_roborio: bool) -> &mut Self {
        self.set(Self::RESTART_ROBORIO, restart_roborio);
        self
    }

    pub fn set_restart_roborio_code(&mut self, restart_roborio_code: bool) -> &mut Self {
        self.set(Self::RESTART_ROBORIO_CODE, restart_roborio_code);
        self
    }
}

impl RobotRequestCode {
    pub fn to_bits(&self) -> u8 {
        self.0
    }
}

mycelium_bitfield::bitfield! {
    #[derive(Default, Eq, PartialEq, Hash)]
    pub struct DriverstationRequestCode<u8>{
        pub const REQUEST_TIME: bool;
        pub const REQUEST_DISABLE: bool;
        // const _RESERVED1 = 6;
        // pub const _2: bool;
        pub const _3: bool;
        pub const _4: bool;
        pub const _5: bool;
        pub const _6: bool;
        pub const _7: bool;
        pub const _8: bool;
    }
}

impl DriverstationRequestCode {
    pub fn request_time(&self) -> bool {
        self.get(Self::REQUEST_TIME)
    }

    pub fn request_disabled(&self) -> bool {
        self.get(Self::REQUEST_DISABLE)
    }

    pub fn request_whatever_makes_the_robot_not_enable(&self) -> bool {
        self.get(Self::REQUEST_DISABLE)
    }

    pub fn set_request_time(&mut self, request: bool) -> &mut Self {
        self.set(Self::REQUEST_TIME, request);
        self
    }

    pub fn set_request_disable(&mut self, disable: bool) -> &mut Self {
        self.set(Self::REQUEST_DISABLE, disable);
        self
    }
}

impl DriverstationRequestCode {
    pub fn to_bits(&self) -> u8 {
        self.0
    }
}

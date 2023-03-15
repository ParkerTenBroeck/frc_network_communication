mycelium_bitfield::bitfield! {
    #[derive(Default, PartialEq, Eq, Hash)]
    pub struct ControlCode<u8> {
        pub const MODE = 2;
        pub const ENABLED: bool;
        pub const FMS_ATTACHED: bool;
        pub const BROWN_OUT_PROTECTION: bool;
        pub const _RESERVED = 1;
        pub const DS_ATTACHED: bool;
        pub const ESTOP: bool;
    }
}

impl ControlCode {
    pub fn to_bits(&self) -> u8 {
        self.0
    }

    pub fn is_invalid(&self) -> bool {
        self.get(Self::_RESERVED) > 0
    }

    pub fn is_test(&self) -> bool {
        self.get(Self::MODE) == 1
    }

    pub fn is_autonomus(&self) -> bool {
        self.get(Self::MODE) == 2
    }

    pub fn is_practise(&self) -> bool {
        // self.get(Self::P)
        todo!()
    }

    pub fn is_teleop(&self) -> bool {
        self.get(Self::MODE) == 3
    }

    pub fn is_estop(&self) -> bool {
        self.get(Self::ESTOP)
    }

    pub fn is_enabled(&self) -> bool {
        self.get(Self::ENABLED)
    }

    pub fn is_disabled(&self) -> bool {
        !self.is_enabled()
    }

    pub fn is_brown_out_protection(&self) -> bool {
        self.get(Self::BROWN_OUT_PROTECTION)
    }

    pub fn is_driverstation_attached(&self) -> bool {
        self.get(Self::DS_ATTACHED)
    }

    pub fn set_test(&mut self) -> &mut Self {
        self.set(Self::MODE, 1);
        self
    }

    pub fn set_autonomus(&mut self) -> &mut Self {
        self.set(Self::MODE, 2);
        self
    }

    pub fn set_teleop(&mut self) -> &mut Self {
        self.set(Self::MODE, 0);
        self
    }

    pub fn set_enabled(&mut self) -> &mut Self {
        self.set(Self::ENABLED, true);
        self
    }

    pub fn set_disabled(&mut self) -> &mut Self {
        self.set(Self::ENABLED, false);
        self
    }

    pub fn set_brownout_protection(&mut self, active: bool) -> &mut Self {
        self.set(Self::BROWN_OUT_PROTECTION, active);
        self
    }

    pub fn set_estop(&mut self, estop: bool) -> &mut Self {
        self.set(Self::ESTOP, estop);
        self
    }

    pub fn set_fms_attached(&mut self, fms_attached: bool) -> &mut Self {
        self.set(Self::FMS_ATTACHED, fms_attached);
        self
    }

    pub fn set_ds_attached(&mut self, attached: bool) -> &mut Self {
        self.set(Self::DS_ATTACHED, attached);
        self
    }
}

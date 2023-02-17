macro_rules! prim_to_enum {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($(#[$vmeta:meta])* $vname:ident = $val:expr,)*
        [unknown] $(#[$unknown_meta2:meta])* $unknown_name:ident ($type:ident) $(= $unknown_val:expr)?$(,)?
    }) => {
        $(#[$meta])*
        $vis enum $name {
            $($(#[$vmeta])* $vname = $val,)*
            $(#[$unknown_meta2])* $unknown_name ($type) $(= $unknown_val)?,
        }

        impl std::convert::From<i32> for $name {
            fn from(v: i32) -> Self {
                match v {
                    $($val => $name::$vname,)*
                    val => $name::$unknown_name(val),
                }
            }
        }

        impl std::convert::From<$name> for i32{
            fn from(v: $name) -> i32{
                match v{
                    $($name::$vname => $val,)*
                    $name::$unknown_name(val) => val,
                }
            }
        }

        impl std::fmt::Display for $name{
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
                std::write!(f, "{:?}", self)
            }
        }
    }
}

prim_to_enum!(
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(i32)]
    pub enum Warnings {
        SampleRateTooHigh = 1,
        VoltageOutOfRange = 2,
        CompressorTaskError = 3,
        LoopTimingError = 4,
        NonBinaryDigitalValue = 5,
        IncorrectBatteryChannel = 6,
        BadJoystickIndex = 7,
        BadJoystickAxis = 8,
        InvalidMotorIndex = 9,
        DriverStationTaskError = 10,
        EnhancedIOPWMPeriodOutOfRange = 11,
        SPIWriteNoMOSI = 12,
        SPIReadNoMISO = 13,
        SPIReadNoData = 14,
        IncompatibleState = 15,

        [unknown]
        Unknown(i32) = i32::MAX
    }
);

prim_to_enum!(
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(i32)]
    pub enum Errors {
        ModuleIndexOutOfRange = -1,
        ChannelIndexOutOfRange = -45,
        NotAllocated = -2,
        ResourceAlreadyAllocated = -3,
        NoAvailableResources = -4,
        NullParameter = -5,
        Timeout = -6,
        CompassManufacturerError = -7,
        CompassTypeError = -8,
        IncompatibleMode = -9,
        AnalogTriggerLimitOrderError = -10,
        AnalogTriggerPulseOutputError = -11,
        TaskError = -12,
        TaskIDError = -13,
        TaskDeletedError = -14,
        TaskOptionsError = -15,
        TaskMemoryError = -16,
        TaskPriorityError = -17,
        DriveUninitialized = -18,
        CompressorNonMatching = -19,
        CompressorAlreadyDefined = -20,
        CompressorUndefined = -21,
        InconsistentArrayValueAdded = -22,
        MismatchedComplexTypeClose = -23,
        DashboardDataOverflow = -24,
        DashboardDataCollision = -25,
        EnhancedIOMissing = -26,
        LineNotOutput = -27,
        ParameterOutOfRange = -28,
        SPIClockRateTooLow = -29,
        JaguarVersionError = -30,
        JaguarMessageNotFound = -31,
        NetworkTablesReadError = -40,
        NetworkTablesBufferFull = -41,
        NetworkTablesWrongType = -42,
        NetworkTablesCorrupt = -43,
        SmartDashboardMissingKey = -44,
        CommandIllegalUse = -50,
        UnsupportedInSimulation = -80,
        CameraServerError = -90,
        InvalidParameter = -100,
        AssertionFailure = -110,
        Error = -111,

        [unknown]
        Unknown(i32) = i32::MIN,
    }
);
